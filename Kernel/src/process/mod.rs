use crate::{
	cpu::syscalls::{self, Registers},
	mem::paging::{self, UserPageTable},
};
use alloc::collections::VecDeque;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::VirtAddr;

type Pid = usize;

lazy_static! {
	static ref QUEUE: Mutex<VecDeque<Pid>> = Mutex::new(VecDeque::new());
}

lazy_static! {
	static ref MAP: Mutex<HashMap<Pid, PCB>> = Mutex::new(HashMap::new());
}

/// Module for working with elf executables
pub mod elf;

// type OpenFiles = ();

enum State {
	New { start: VirtAddr, stack: VirtAddr },
	Running { registers: Registers },
}

/// Process control block
pub struct PCB {
	// pid: Pid,
	state: State,
	page_table: UserPageTable,
	// registers: (),
	// open_files: OpenFiles,
}

impl PCB {
	fn run_a(&self) {
		// Switch to process page table
		unsafe {
			serial_println!("Switching table");
			paging::set_page_table(&self.page_table.0);
			serial_println!("Switched table");
		}
		match self.state {
			State::New { start, stack } => unsafe {
				serial_println!("Going to ring3 - start: {:?} stack: {:?}", start, stack);
				syscalls::go_to_ring3(start, stack);
			},
			State::Running { registers } => {
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
	let temp_stack: *const u8 = unsafe { crate::cpu::gdt::STACK.as_ptr() };
	// Switch to kernel stack
	unsafe {
		asm!("mov rsp, {stack}",
		stack = in(reg) temp_stack)
	}

	loop {
		let pid = QUEUE.lock()[0];
		let lock = MAP.lock();
		let process = lock.get(&pid).expect("process from queue not in hashmap");
		unsafe {
			MAP.force_unlock();
		}
		process.run_a();
	}
}

/// Remvoe the currently running process
pub fn remove_current_process() {
	let mut lock = QUEUE.lock();
	let pid = lock.pop_front().expect("No processes");

	let prev_value = MAP.lock().remove(&pid);
	assert!(prev_value.is_some());

	if lock.is_empty() {
		serial_println!("bye bye");
		crate::end();
	}

	drop(lock);

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

/// Context switch to next process
pub fn context_switch(registers: &Registers) {
	// TODO save state of last process
	{
		let pid: Pid = QUEUE.lock()[0];
		let mut lock = MAP.lock();
		let mut process = lock.get_mut(&pid).expect("process from queue not in hashmap");
		process.state = State::Running { registers: *registers };
	}

	{
		let mut lock = QUEUE.lock();
		serial_println!("lock acquired");
		let popped = lock.pop_front().expect("No processes in queue");
		lock.push_back(popped);
	}

	serial_println!("running next process");
	run_next_process()
}

/// Add a new process to the queue
pub fn add_process(executable_path: &str) -> Result<(), elf::ElfErr> {
	let process = create_process(executable_path)?;
	let new_pid = get_new_pid();

	QUEUE.lock().push_back(new_pid);
	let prev_key = MAP.lock().insert(new_pid, process);
	assert!(prev_key.is_none());
	Ok(())
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

fn create_process(executable_path: &str) -> Result<PCB, elf::ElfErr> {
	serial_println!("Creating process: {}", executable_path);
	let mut page_table = paging::get_new_user_table();
	let (start, stack) = elf::load_elf(executable_path, &mut page_table.0)?;
	Ok(PCB {
		state: State::New { stack, start },
		page_table,
	})
}
