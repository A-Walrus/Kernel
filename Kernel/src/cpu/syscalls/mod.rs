use core::{ptr::slice_from_raw_parts, str};

use crate::{cpu::gdt::GDT, print, serial_print, serial_println};
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

const SYSCALLS: [Syscall; 2] = [sys_debug, sys_print];

fn sys_print(ptr: u64, len: u64, _: u64, _: u64, _: u64, _: u64) -> SyscallResult {
	let ptr = ptr as *const u8;
	// This is not sound. Who knows wha the user put as the pointer
	let opt_slice;
	unsafe {
		opt_slice = slice_from_raw_parts(ptr, len as usize).as_ref();
	}
	if let Some(slice) = opt_slice {
		let a = str::from_utf8(slice);
		if let Ok(s) = a {
			print!("{}", s);
			return SyscallResult(0);
		} else {
			return SyscallResult(-1);
		}
	} else {
		return SyscallResult(-1); // Failiure
	}
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
struct ScratchRegisters {
	r11: u64,
	r10: u64,
	r9: u64,
	r8: u64,
	rsi: u64,
	rdi: u64,
	rdx: u64,
	rcx: u64,
	rax: i64,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct PreservedRegisters {
	r15: u64,
	r14: u64,
	r13: u64,
	r12: u64,
	rbp: u64,
	rbx: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Registers {
	preserved: PreservedRegisters,
	scratch: ScratchRegisters,
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

// fn resume_program_execution(registers: Registers) {
// 	// Or reference / pointer
// 	unsafe {
// 		asm!(
// 			"mov rbp, rax",
// 			"mov rbx, rdx",
// 			in("rax") registers.preserved.rbp,
// 			in("rdx") registers.preserved.rbx,
// 			// TODO figure out if i need to say I'm changing rbp
// 			options(noreturn)
// 		);

// 		asm!(
// 			"",
// 			// in("rbx") registers.preserved.rbx, //moved through rdx before
// 			// in("rbp") registers.preserved.rbp, // Moved through rax before
// 			in("r12") registers.preserved.r12,
// 			in("r13") registers.preserved.r13,
// 			in("r14") registers.preserved.r14,
// 			in("r15") registers.preserved.r15,
// 			in("r11") registers.scratch.r11,
// 			in("r10") registers.scratch.r10,
// 			in("r9") registers.scratch.r9,
// 			in("r8") registers.scratch.r8,
// 			in("rsi") registers.scratch.rsi,
// 			in("rdi") registers.scratch.rdi,
// 			in("rdx") registers.scratch.rdx,
// 			in("rcx") registers.scratch.rcx,
// 			in("rax") registers.scratch.rax,
// 			options(noreturn)
// 		);
// 		asm!("sysretq", options(noreturn))
// 	}
// 	unimplemented!()
// }

#[naked]
extern "C" fn handle_syscall() {
	unsafe {
		asm!(
			// Push scratch registers
			"
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
			pop rax",
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
