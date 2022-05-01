use core::{ptr::slice_from_raw_parts, str};

use crate::{cpu::gdt::GDT, print, process, serial_print, serial_println};
use x86_64::{
	instructions::segmentation::DS,
	registers::{model_specific::*, rflags::RFlags},
	VirtAddr,
};

// On enter
// rax  system call number
// rcx  return address
// r11  saved rflags (note: r11 is callee-clobbered register in C ABI)
// rdi  arg0
// rsi  arg1
// rdx  arg2
// r10  arg3 (needs to be moved to rcx to conform to C ABI)
// r8   arg4
// r9   arg5
// (note: r12-r15, rbp, rbx are callee-preserved in C ABI)
// RAX - syscall num
// RCX - return address
// R11 saved rflags
// RDI arg

#[repr(transparent)]
/// Result of a syscall
pub struct SyscallResult(i64);

/// A system call function
pub type Syscall = fn(arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> SyscallResult;

const SYSCALLS: [Syscall; 4] = [sys_debug, sys_print, sys_exit, sys_exec];

fn sys_print(ptr: u64, len: u64, _: u64, _: u64, _: u64, _: u64) -> SyscallResult {
	let ptr = ptr as *const u8;
	serial_println!("sys_print, ptr: {:?}, len: {}", ptr, len);
	let opt_slice;
	unsafe {
		// This is not sound. Who knows what the user put as the pointer
		opt_slice = slice_from_raw_parts(ptr, len as usize).as_ref();
	}
	if let Some(slice) = opt_slice {
		let a = str::from_utf8(slice);
		if let Ok(s) = a {
			print!("{}", s);
			return SyscallResult(0);
		} else {
			serial_println!("Invalid UTF print");
			return SyscallResult(-1);
		}
	} else {
		return SyscallResult(-1); // Failiure
	}
}

fn sys_exec(ptr: u64, len: u64, _: u64, _: u64, _: u64, _: u64) -> SyscallResult {
	let ptr = ptr as *const u8;
	serial_println!("sys_exec, ptr: {:?}, len: {}", ptr, len);
	let opt_slice;
	unsafe {
		// This is not sound. Who knows what the user put as the pointer
		opt_slice = slice_from_raw_parts(ptr, len as usize).as_ref();
	}

	if let Some(slice) = opt_slice {
		let a = str::from_utf8(slice);
		if let Ok(s) = a {
			serial_println!("Add process");
			let res = crate::process::add_process(s);
			serial_println!("Added process");
			match res {
				Ok(pid) => SyscallResult(pid as u32 as i64),
				Err(_e) => SyscallResult(-1),
			}
		} else {
			serial_println!("Invalid UTF print");
			SyscallResult(-1)
		}
	} else {
		SyscallResult(-1) // Failiure
	}
}

// fn sys_open_file(ptr: u64, len: u64, _: u64, _: u64, _: u64, _: u64) -> SyscallResult {
// 	// This is not implemented
// 	unimplemented!();

// 	let ptr = ptr as *const u8;
// 	// This is not sound. Who knows wha the user put as the pointer
// 	let opt_slice;
// 	unsafe {
// 		opt_slice = slice_from_raw_parts(ptr, len as usize).as_ref();
// 	}
// 	if let Some(slice) = opt_slice {
// 		let a = str::from_utf8(slice);
// 		if let Ok(path) = a {
// 			return SyscallResult(0);
// 		} else {
// 			return SyscallResult(-1);
// 		}
// 	} else {
// 		return SyscallResult(-1); // Failiure
// 	}
// }

fn sys_exit(status: u64, _: u64, _: u64, _: u64, _: u64, _: u64) -> SyscallResult {
	serial_println!("Process exited with status: {}", status);
	process::remove_current_process();

	SyscallResult(0) // I think this doesn't matter
}

fn sys_debug(arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> SyscallResult {
	serial_println!("DEBUG SYSCALL, arguments:");
	serial_println!(
		"{:#x}, {:#x}, {:#x}, {:#x}, {:#x}, {:#x}",
		arg0,
		arg1,
		arg2,
		arg3,
		arg4,
		arg5
	);
	return SyscallResult(0); // Success
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
/// Scratch registers
pub struct ScratchRegisters {
	/// r11 register
	pub r11: u64,
	/// r10 register
	pub r10: u64,
	/// r9 register
	pub r9: u64,
	/// r8 register
	pub r8: u64,
	/// rsi register
	pub rsi: u64,
	/// rdi register
	pub rdi: u64,
	/// rdx register
	pub rdx: u64,
	/// rcx register
	pub rcx: u64,
	/// rax register
	pub rax: i64,
	/// rsp register
	pub rsp: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
/// Preserved registers
pub struct PreservedRegisters {
	/// r15 register
	pub r15: u64,
	/// r14 register
	pub r14: u64,
	/// r13 register
	pub r13: u64,
	/// r12 register
	pub r12: u64,
	/// rbp register
	pub rbp: u64,
	/// rbx register
	pub rbx: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
/// Registers
pub struct Registers {
	/// Preserved registers
	pub preserved: PreservedRegisters,
	/// Scratch registers
	pub scratch: ScratchRegisters,
}

#[allow(dead_code)] // called from asm
#[no_mangle] // called from asm
extern "C" fn handle_syscall_inner(registers_ptr: *mut Registers) {
	serial_println!("HANDLING SYSCALL");

	let registers: &mut Registers;
	unsafe {
		registers = &mut *registers_ptr;
	}

	let function = SYSCALLS.get(registers.scratch.rax as usize);
	match function {
		Some(func) => {
			let scratch = &mut registers.scratch;
			let rdi = scratch.rdi;
			let rsi = scratch.rsi;
			let rdx = scratch.rdx;
			let r8 = scratch.r8;
			let r9 = scratch.r9;
			let r10 = scratch.r10;
			let result = func(rdi, rsi, rdx, r10, r8, r9);

			scratch.rax = result.0;

			// TODO figure this out
			crate::process::context_switch(registers);
			return;
		}
		None => {
			unimplemented!("INVALID SYSCALL");
		}
	}
	// TODO cleanup stack
}

#[no_mangle] // called from asm
extern "C" fn do_nothing() {
	serial_print!("HANDLING SYSCALL");
}

#[naked]
extern "C" fn handle_syscall() {
	unsafe {
		asm!(
			// Push scratch registers
			"
			push rsp
			push rax
			push rcx
			push rdx
			push rdi
			push rsi
			push r8
			push r9
			push r10
			push r11",
			// Push preserved registers
			"
			push rbx
			push rbp
			push r12
			push r13
			push r14
			push r15
			",
			"mov rdi, rsp", // C calling convention first variable
			// "add rsp, 8",
			"call handle_syscall_inner",
			// Pop preserved registers
			"
			pop r15
			pop r14
			pop r13
			pop r12
			pop rbp
			pop rbx",
			// Pop scratch registers
			"
			pop r11
			pop r10
			pop r9
			pop r8
			pop rsi
			pop rdi
			pop rdx
			pop rcx
			pop rax
			pop rsp",
			"sysretq",
			options(noreturn)
		);
	}
}

/// Setup for system calls
pub fn setup() {
	let cs_sysret = GDT.1.user_code_selector;
	let ss_sysret = GDT.1.user_data_selector;
	let cs_syscall = GDT.1.kernel_code_selector;
	let ss_syscall = GDT.1.kernel_data_selector;
	Star::write(cs_sysret, ss_sysret, cs_syscall, ss_syscall).expect("Failed to write MSR Star");

	unsafe {
		// Enable syscalls through Efer MSR
		Efer::write(Efer::read() | EferFlags::SYSTEM_CALL_EXTENSIONS);
	}

	// Write syscall handler address to LSTAR MSR
	LStar::write(VirtAddr::from_ptr(handle_syscall as *const u8));

	// Not sure what this does...
	SFMask::write(RFlags::INTERRUPT_FLAG);
}

/// Go to ring 3 with given code and stack addresses
pub unsafe fn go_to_ring3(code: VirtAddr, stack_end: VirtAddr) {
	let cs_idx: u16 = GDT.1.user_code_selector.0;
	let ds_idx: u16 = GDT.1.user_data_selector.0;

	use x86_64::instructions::segmentation::Segment;
	x86_64::instructions::tlb::flush_all();
	DS::set_reg(GDT.1.user_data_selector);
	asm!(
	"push rax",
	"push rsi",
	"push 0x200",
	"push rdx",
	"push rdi",
	"iretq",
	in("rdi") code.as_u64(),
	in("rsi") stack_end.as_u64(),
	in("dx") cs_idx,
	in("ax") ds_idx,
	);
}
