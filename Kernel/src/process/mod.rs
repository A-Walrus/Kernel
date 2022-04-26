use crate::{
	cpu::syscalls::{self, Registers},
	mem::paging::{self, UserPageTable},
};
use alloc::{boxed::Box, collections::VecDeque};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::{structures::paging::PageTable, VirtAddr};

lazy_static! {
	static ref QUEUE: Mutex<VecDeque<PCB>> = Mutex::new(VecDeque::new());
}

/// Module for working with elf executables
pub mod elf;

// type Pid = usize;

// type OpenFiles = ();

enum State {
	New { start: VirtAddr, stack: VirtAddr },
	Running { registers: () },
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
	fn run(&self) {
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
			_ => unimplemented!(),
		}
	}
}

/// Run the next process in the queue
pub fn run_next_process() {
	loop {
		let lock = QUEUE.lock();
		let process = lock.front().expect("No processes in queue");
		unsafe {
			QUEUE.force_unlock();
		}
		process.run();
	}
}

/// Context switch to next process
pub fn context_switch(registers: &Registers) {
	// TODO save state of last process

	serial_println!("Context switching!");
	let temp_stack: *const u8 = unsafe { crate::cpu::gdt::STACK.as_ptr() };
	// Switch to kernel stack
	unsafe {
		asm!("mov rsp, {stack}",
		stack = in(reg) temp_stack)
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
	QUEUE.lock().push_back(process);
	Ok(())
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
