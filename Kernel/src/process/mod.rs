use crate::{
	cpu::syscalls::{self, OpenFlags, Registers},
	fs::ext2::{Directory, Entry, Ext2Err, File},
	mem::paging::{self, UserPageTable},
	util::io::{IOError, Read, Seek, Write},
};
use alloc::{
	collections::VecDeque,
	string::String,
	vec::{IntoIter, Vec},
};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use spin::Mutex;

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

enum BackHandle {
	File(File),
	Dir(IntoIter<Entry>),
}

/// A struct managing open files
pub struct OpenFiles {
	handles: HashMap<Handle, BackHandle>,
	next: Handle, // handles: (),
}

unsafe impl Send for PCB {}
unsafe impl Sync for PCB {}

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

enum State {
	New(elf::LoadData),
	Running { registers: Registers },
}

/// Data needed when unblocking process
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
pub struct PCB {
	// pid: Pid,
	state: State,
	block_state: BlockState,
	page_table: UserPageTable,
	/// Input buffer for the process
	pub input_buffer: String,
	/// This processes open files
	pub open_files: OpenFiles,
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

	fn try_run(&mut self) {
		// Switch to process page table
		unsafe {
			paging::set_page_table(&self.page_table.0);
		}
		match self.state {
			State::New(data) => unsafe {
				// serial_println!("Going to ring3 - start: {:?} stack: {:?}", start, stack);
				syscalls::go_to_ring3(data.entry, data.stack_top, data.argc, data.argv.as_u64() as usize);
			},
			State::Running { registers } => {
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
pub fn run_next_process() {
	loop {
		x86_64::instructions::interrupts::disable();
		let len = QUEUE.lock().len();
		for _ in 0..len {
			let pid = QUEUE.lock()[0];
			let mut lock = MAP.lock();
			let process = lock.get_mut(&pid).expect("process from queue not in hashmap");
			if process.block_state.ready() {
				unsafe {
					MAP.force_unlock();
				}
				process.try_run();
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
pub fn remove_current_process() {
	{
		let mut lock = QUEUE.lock();
		let pid = lock.pop_front().expect("No processes");

		let prev_value = MAP.lock().remove(&pid);
		assert!(prev_value.is_some());

		if lock.is_empty() {
			crate::end();
		}
	}

	run_next_process();
}

/// Remove a process from running
pub fn remove_process(pid: Pid) {
	let prev_value = MAP.lock().remove(&pid);
	assert!(prev_value.is_some());

	let mut lock = QUEUE.lock();
	let index = lock.iter().position(|x| *x == pid).unwrap();
	let prev_value = lock.remove(index);
	assert!(prev_value.is_some());
}

fn cycle() {
	{
		let mut lock = QUEUE.lock();
		let popped = lock.pop_front().expect("No processes in queue");
		lock.push_back(popped);
	}
}

/// Context switch to next process
pub fn context_switch(registers: &Registers) {
	{
		let pid: Pid = QUEUE.lock()[0];
		let mut lock = MAP.lock();
		let mut process = lock.get_mut(&pid).expect("process from queue not in hashmap");
		process.state = State::Running { registers: *registers };
	}
	cycle();

	run_next_process()
}

/// Add a new process to the queue
pub fn add_process(executable_path: &str, args: &[&str]) -> Result<Pid, elf::ElfErr> {
	let process = create_process(executable_path, args)?;
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

fn create_process(executable_path: &str, args: &[&str]) -> Result<PCB, elf::ElfErr> {
	serial_println!("Creating process: {}", executable_path);
	let mut page_table = paging::get_new_user_table();
	let data = elf::load_elf(executable_path, &mut page_table.0, args)?;
	Ok(PCB {
		state: State::New(data),
		input_buffer: String::new(),
		block_state: BlockState::Ready,
		open_files: OpenFiles::new(),
		page_table,
	})
}
