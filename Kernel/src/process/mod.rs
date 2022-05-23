use crate::{
	cpu::syscalls::{self, OpenFlags, Registers},
	fs::ext2::{Directory, Entry, Ext2Err, File},
	mem::paging::{self, UserPageTable},
	util::io::{IOError, Read, Write},
};
use alloc::{
	collections::VecDeque,
	string::String,
	vec::{IntoIter, Vec},
};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::VirtAddr;

/// Are processes running
pub static mut RUNNING: bool = false;

/// An identifier for a process. This is unique per process
pub type Pid = usize;

lazy_static! {
	static ref QUEUE: Mutex<VecDeque<Pid>> = Mutex::new(VecDeque::new());
}

lazy_static! {
	/// Hashmap containg PCBs of processes by Pid
	pub static ref MAP: Mutex<HashMap<Pid, PCB>> = Mutex::new(HashMap::new());
}

/// Module for working with elf executables
pub mod elf;

#[derive(Debug)]
enum BackHandle {
	File(File),
	Dir(IntoIter<Entry>),
}

/// A struct managing open files
#[derive(Debug)]
pub struct OpenFiles {
	handles: HashMap<Handle, BackHandle>,
	next: Handle, // handles: (),
}

unsafe impl Send for PCB {}
unsafe impl Sync for PCB {}

/// Start process execution loop
pub fn start() {
	// unsafe {
	// RUNNING = true;
	// }
	run_next_process();
}

/// File handle
pub type Handle = u32;
impl OpenFiles {
	fn new() -> Self {
		Self {
			handles: HashMap::new(),
			next: 0,
		}
	}

	/// Read from the file handle into the slice
	pub fn read(&mut self, handle: Handle, slice: &mut [u8]) -> Result<usize, Ext2Err> {
		let back_handle = self.handles.get_mut(&handle).ok_or(Ext2Err::NoHandle)?;
		match back_handle {
			BackHandle::File(file) => Ok(file.read(slice)?),
			BackHandle::Dir(dir) => {
				// if dir.is_empty() {
				// 	return Err(Ext2Err::EndOfDir);
				// }
				match dir.as_slice().first() {
					Some(entry) => {
						let name = &entry.name;
						let len = name.len();
						if len > slice.len() {
							return Err(Ext2Err::IO(IOError::BufferTooSmall));
						}
						serial_println!("{}", name);
						slice[..len].copy_from_slice(name.as_bytes());
						dir.next();
						Ok(len)
					}
					None => Err(Ext2Err::EndOfDir),
				}
			}
		}
	}

	/// Write from the slice to the file handle
	pub fn write(&mut self, handle: Handle, slice: &[u8]) -> Result<usize, Ext2Err> {
		let back_handle = self.handles.get_mut(&handle).ok_or(Ext2Err::NoHandle)?;
		match back_handle {
			BackHandle::File(file) => Ok(file.write(slice)?),
			BackHandle::Dir(_) => Err(Ext2Err::NotAFile),
		}
	}

	/// Open a file, creting a handle
	pub fn open_file(&mut self, path: &str, flags: OpenFlags) -> Result<Handle, Ext2Err> {
		let file = File::from_path(path, flags)?;
		let handle = self.next;
		self.next += 1;
		let prev = self.handles.insert(handle, BackHandle::File(file));
		assert!(prev.is_none());
		Ok(handle)
	}

	/// Open a directory, creting a handle
	pub fn open_dir(&mut self, path: &str) -> Result<Handle, Ext2Err> {
		let directory = Directory::from_path(path)?;
		let handle = self.next;
		self.next += 1;
		let prev = self
			.handles
			.insert(handle, BackHandle::Dir(directory.entries.into_iter()));
		assert!(prev.is_none());
		Ok(handle)
	}

	/// Close a handle to a file
	pub fn close(&mut self, handle: Handle) -> Result<(), Ext2Err> {
		// self.handles.get_mut(&handle).ok_or(Ext2Err::NoHandle)?;
		let result = self.handles.remove(&handle);
		match result {
			Some(_) => Ok(()),
			None => Err(Ext2Err::NoHandle),
		}
	}
}

#[derive(Debug)]
/// State of process
pub enum State {
	/// new process
	New(elf::LoadData),
	/// process stopped by syscall
	Syscall {
		/// saved registers
		registers: Registers,
	},
	/// process stopped by timer
	Timer {
		/// saved registers
		registers: Registers,
		/// saved RIP
		instruction_pointer: VirtAddr,
		/// saved rflags
		rflags: u64,
	},
}

/// Data needed when unblocking process
#[derive(Debug)]
pub enum BlockData {
	/// data for input syscall
	Input {
		/// slice to write to
		slice: *mut [u8],
	},
	/// Waiting for aprocess to finish
	Wait(Pid),
}

unsafe impl Sync for BlockData {}
unsafe impl Send for BlockData {}

#[derive(Debug)]
enum BlockState {
	Blocked { still: bool, data: BlockData },
	Ready,
}

impl BlockState {
	fn ready(&self) -> bool {
		match self {
			BlockState::Ready => true,
			BlockState::Blocked { still: false, data: _ } => true,
			_ => false,
		}
	}
}

/// Process control block
#[derive(Debug)]
pub struct PCB {
	// pid: Pid,
	state: State,
	block_state: BlockState,
	page_table: UserPageTable,
	/// Input buffer for the process
	pub input_buffer: String,
	/// This processes open files
	pub open_files: OpenFiles,
	waiting_processes: Vec<Pid>,
	/// Terminal this process prints to
	pub terminal: usize,
}

/// Get process in foreground
pub fn foreground_process() -> Pid {
	// TODO actual foreground
	QUEUE.lock()[0]
}

/// Get currenty running process
pub fn running_process() -> Pid {
	QUEUE.lock()[0]
}

/// Block the currently running process
pub fn block_current(data: BlockData) {
	let pid = running_process();
	MAP.lock()
		.get_mut(&pid)
		.expect("running process not in queue")
		.block_state = BlockState::Blocked { still: true, data };
}

impl PCB {
	/// append input to this processes input buffer
	pub fn append_input(&mut self, character: char) {
		self.input_buffer.push(character);
		match &mut self.block_state {
			BlockState::Blocked {
				still,
				data: BlockData::Input { slice: _ },
			} => {
				*still = false;
			}
			_ => {}
		}
	}

	/// Append a process to waiting processes
	pub fn append_waiting(&mut self, pid: Pid) {
		self.waiting_processes.push(pid);
	}

	fn run_proc(&mut self) {
		unsafe {
			RUNNING = true;
			crate::cpu::interrupts::COUNTER = 0;
		};

		// Switch to process page table
		unsafe {
			paging::set_page_table(&self.page_table.0);
		}
		match self.state {
			State::New(data) => unsafe {
				// serial_println!("Going to ring3 - start: {:?} stack: {:?}", start, stack);
				syscalls::go_to_ring3(data.entry, data.stack_top, data.argc, data.argv.as_u64() as usize);
			},
			State::Timer {
				registers,
				instruction_pointer,
				rflags,
			} => {
				// serial_println!("restoring {:?}", registers);
				unsafe {
					use crate::cpu::gdt::GDT;
					let cs_idx: u16 = GDT.1.user_code_selector.0;
					let ds_idx: u16 = GDT.1.user_data_selector.0;

					use x86_64::instructions::segmentation::{Segment, DS};
					DS::set_reg(GDT.1.user_data_selector);

					asm!(
						"push rax",
						"push {sp}",
						"push {flags}",
						"push rdx",
						"push {ip}",
						"add rsp, 8*5",
						sp = in(reg) registers.scratch.rsp,
						flags = in(reg) rflags,
						ip = in(reg) instruction_pointer.as_u64(),
						in("dx") cs_idx,
						in("ax") ds_idx,

					);

					asm!(
						"mov rbp, {rbp}",
						"mov rbx, {rbx}",
						rbp = in(reg) registers.preserved.rbp,
						rbx = in(reg) registers.preserved.rbx,
						// TODO figure out if i need to say I'm changing rbp
					);
					asm!(
						"",
						in("r12") registers.preserved.r12,
						in("r13") registers.preserved.r13,
						in("r14") registers.preserved.r14,
						in("r15") registers.preserved.r15,
						in("r11") registers.scratch.r11,
						in("r10") registers.scratch.r10,
						in("r9") registers.scratch.r9,
						in("r8") registers.scratch.r8,
						in("rsi") registers.scratch.rsi,
						in("rax") registers.scratch.rax,
						in("rdi") registers.scratch.rdi,
						in("rdx") registers.scratch.rdx,
						in("rcx") registers.scratch.rcx,
					);

					asm!("sub rsp, 8*5", "iretq");
				}
			}
			State::Syscall { registers } => {
				if let BlockState::Blocked {
					still: false,
					data: BlockData::Input { slice },
				} = self.block_state
				{
					let slice = unsafe { slice.as_mut().unwrap() };
					use core::cmp::min;

					let buffer: &mut String = &mut self.input_buffer;

					let amount_to_take = min(buffer.len(), slice.len());
					slice[..amount_to_take].copy_from_slice(&buffer.as_bytes()[..amount_to_take]);

					buffer.drain(0..amount_to_take);
				}

				// address to temporarily store rax while fixing the stack.
				let addr = registers.scratch.rsp - 0x08;

				unsafe {
					asm!(
						"mov rbp, {rbp}",
						"mov rbx, {rbx}",
						rbp = in(reg) registers.preserved.rbp,
						rbx = in(reg) registers.preserved.rbx,
						// TODO figure out if i need to say I'm changing rbp
					);
					asm!("mov QWORD PTR [{addr}],rax",
						addr = in(reg) addr,
						in("rax") registers.scratch.rax);

					asm!(
						"",
						in("r12") registers.preserved.r12,
						in("r13") registers.preserved.r13,
						in("r14") registers.preserved.r14,
						in("r15") registers.preserved.r15,
						in("r11") registers.scratch.r11,
						in("r10") registers.scratch.r10,
						in("r9") registers.scratch.r9,
						in("r8") registers.scratch.r8,
						in("rsi") registers.scratch.rsi,
						in("rdi") registers.scratch.rdi,
						in("rdx") registers.scratch.rdx,
						in("rcx") registers.scratch.rcx,
					);
					asm!(
						"push rax",
						"pop rsp",
						"mov rax, QWORD PTR [rsp-0x08]", // rax from when it was pushed before
						"sysretq",
						in("rax") registers.scratch.rsp,
						options(noreturn)
					);
				}
			}
		}
	}
}

/// Run the next process in the queue
pub fn run_next_process() -> ! {
	unsafe {
		RUNNING = false;
	}
	loop {
		x86_64::instructions::interrupts::disable();
		let len = QUEUE.lock().len();
		for _ in 0..len {
			let pid = QUEUE.lock()[0];
			let mut lock = MAP.lock();
			// serial_print!("{} ", pid);
			let process = lock.get_mut(&pid).expect("process from queue not in hashmap");
			if process.block_state.ready() {
				unsafe {
					MAP.force_unlock();
				}
				process.run_proc();
			} else {
				// serial_println!("blocked");
			}
			cycle();
		}
		x86_64::instructions::interrupts::enable();
		x86_64::instructions::hlt();
	}
}

/// Remvoe the currently running process
pub fn remove_current_process() -> ! {
	let removing_pid: Pid = *QUEUE.lock().front().expect("No processes");
	remove_process(removing_pid);

	run_next_process();
}

/// Remove a process from running
pub fn remove_process(removing_pid: Pid) {
	let mut queue = QUEUE.lock();
	if let Some((index, _)) = queue.iter().enumerate().find(|(_, p)| **p == removing_pid) {
		queue.remove(index);
		{
			let mut lock = MAP.lock();
			let prev_value = lock.remove(&removing_pid);
			assert!(prev_value.is_some());
			for pid in prev_value.unwrap().waiting_processes {
				let process = lock.get_mut(&pid).unwrap();
				match &mut process.block_state {
					BlockState::Blocked {
						still,
						data: BlockData::Wait(waiting_pid),
					} if *waiting_pid == removing_pid => {
						*still = false;
					}
					_ => {}
				}
			}

			if lock.is_empty() {
				crate::end();
			}
		}
	} else {
		serial_println!("fuck");
	}
}

fn cycle() {
	{
		let mut lock = QUEUE.lock();
		let popped = lock.pop_front().expect("No processes in queue");
		lock.push_back(popped);
	}
}

/// Context switch to next process
pub fn context_switch(state: State) -> ! {
	{
		let pid: Pid = QUEUE.lock()[0];
		let mut lock = MAP.lock();
		let mut process = lock.get_mut(&pid).expect("process from queue not in hashmap");
		process.state = state;
	}
	cycle();

	run_next_process()
}

/// Add a new process to the queue
pub fn add_process(executable_path: &str, args: &[&str], term: Option<usize>) -> Result<Pid, elf::ElfErr> {
	let process = create_process(executable_path, args, term)?;
	let new_pid = get_new_pid();

	QUEUE.lock().push_back(new_pid);
	let prev_key = MAP.lock().insert(new_pid, process);
	assert!(prev_key.is_none());
	Ok(new_pid)
}

fn get_new_pid() -> Pid {
	let mut pid = 0;
	let lock = MAP.lock();
	loop {
		if !lock.contains_key(&pid) {
			break;
		}
		pid += 1;
	}
	pid
}

fn create_process(executable_path: &str, args: &[&str], term: Option<usize>) -> Result<PCB, elf::ElfErr> {
	let terminal = term.unwrap_or_else(|| crate::io::buffer::active_term());
	serial_println!("Creating process: {}", executable_path);
	let mut page_table = paging::get_new_user_table();
	let data = elf::load_elf(executable_path, &mut page_table.0, args)?;
	Ok(PCB {
		state: State::New(data),
		input_buffer: String::new(),
		block_state: BlockState::Ready,
		open_files: OpenFiles::new(),
		waiting_processes: Vec::new(),
		page_table,
		terminal,
	})
}
