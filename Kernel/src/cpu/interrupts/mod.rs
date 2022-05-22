use crate::{cpu::syscalls::Registers, println, process, serial_println};
use alloc::vec::Vec;
use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::{structures::idt::PageFaultErrorCode, VirtAddr};

/// Offset of the first pic in the chained pics
pub const PIC_1_OFFSET: u8 = 32;
/// Offset of the second pic in the chained pics
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

const IRQS: usize = 16;

/// Mutex wrapping chained pics. This is the interface for communicating with the pics.
pub static PICS: spin::Mutex<ChainedPics> = spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });
mod codegen;
use codegen::*;

lazy_static! {
	/// Mutex wrapping the interrupt descriptor table
	pub static ref IDT: Mutex<InterruptDescriptorTable> = {
		let mut idt = InterruptDescriptorTable::new();

		unsafe {
			idt.divide_error.set_handler_fn(divide_error_handler).set_stack_index(1);
			idt.debug.set_handler_fn(debug_handler).set_stack_index(1);
			idt.non_maskable_interrupt.set_handler_fn(non_maskable_interrupt_handler).set_stack_index(1);
			idt.breakpoint.set_handler_fn(breakpoint_handler).set_stack_index(1);
			idt.overflow.set_handler_fn(overflow_handler).set_stack_index(1);
			idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler).set_stack_index(1);
			idt.invalid_opcode.set_handler_fn(invalid_opcode_handler).set_stack_index(1);
			idt.device_not_available.set_handler_fn(device_not_available_handler).set_stack_index(1);
			idt.double_fault.set_handler_fn(double_fault_handler).set_stack_index(0);
			idt.invalid_tss.set_handler_fn(invalid_tss_handler).set_stack_index(1);
			idt.segment_not_present.set_handler_fn(segment_not_present_handler).set_stack_index(1);
			idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler).set_stack_index(1);
			idt.general_protection_fault.set_handler_fn(general_protection_fault_handler).set_stack_index(1);
			idt.page_fault.set_handler_fn(page_fault_handler).set_stack_index(1);
			idt.x87_floating_point.set_handler_fn(x87_floating_point_handler).set_stack_index(1);
			idt.alignment_check.set_handler_fn(alignment_check_handler).set_stack_index(1);
			idt.machine_check.set_handler_fn(machine_check_handler).set_stack_index(1);
			idt.simd_floating_point.set_handler_fn(simd_floating_point_handler).set_stack_index(1);
			idt.virtualization.set_handler_fn(virtualization_handler).set_stack_index(1);
			idt.security_exception.set_handler_fn(security_exception_handler).set_stack_index(1);
		}


		set_irq_handlers(&mut idt);

		unsafe {
			idt[(PIC_1_OFFSET + 0) as usize].set_handler_addr(VirtAddr::from_ptr(handle_timer as *const u8)).set_stack_index(1);
			// idt[(PIC_1_OFFSET + 0) as usize].set_handler_fn(test_handler).set_stack_index(1);
		}

		Mutex::new(idt)
	};
}

const QUANTA: usize = 5;

/// count of ticks since starting current process
pub static mut COUNTER: usize = 0;

#[allow(dead_code)] // called from asm
#[no_mangle] // called from asm
extern "C" fn handle_timer_inner(registers_ptr: *mut Registers) -> *const u8 {
	let count;
	unsafe {
		count = COUNTER;
		COUNTER += 1;
	};
	unsafe {
		PICS.lock().notify_end_of_interrupt(PIC_1_OFFSET + 0);
	}
	let running = unsafe { process::RUNNING };
	if running && count >= QUANTA {
		serial_println!("The clock's run out, time's up, over, blaow");

		let registers: &mut Registers;
		unsafe {
			registers = &mut *registers_ptr;
		}
		let stack_frame_ptr = registers.scratch.rsp as *const InterruptStackFrame;
		let stack_frame = unsafe { &*stack_frame_ptr };
		registers.scratch.rsp = stack_frame.stack_pointer.as_u64();
		let registers = *registers;
		let instruction_pointer = stack_frame.instruction_pointer;
		let rflags = stack_frame.cpu_flags;

		// serial_println!("storing: {:?}", registers);
		crate::process::context_switch(process::State::Timer {
			registers,
			instruction_pointer,
			rflags,
		});
	} else {
		return registers_ptr as *const _;
	}
}

const STACK_SIZE: usize = 4096 * 8;
static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

#[allow(dead_code)] // called from asm
#[no_mangle] // called from asm
extern "C" fn get_timer_stack_addr() -> *const u8 {
	// Switch to kernel stack
	let temp_stack: *const u8 = unsafe { STACK.as_ptr().add(STACK_SIZE) };
	return temp_stack;
}

#[naked]
extern "C" fn handle_timer() {
	unsafe {
		asm!(
			// Push scratch registers
			"
			cli
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
			"mov rbx, rsp",              // C calling convention first variable
			"call get_timer_stack_addr", // Hope rbx doesn't get destroyed...
			"mov rsp, rax",
			"mov rdi, rbx", // C calling convention first variable
			"call handle_timer_inner",
			"mov rsp, rax",
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
			"iretq",
			options(noreturn)
		);
	}
}

/// Set up interrupt descriptor table, and chained pics
pub fn setup() {
	unsafe {
		let idt = &mut *(&mut *IDT.lock() as *mut InterruptDescriptorTable);
		idt.load();
	}
	unsafe {
		let mut pics = PICS.lock();
		pics.initialize();
		const MASK: u8 = 0b1111_1000;
		pics.write_masks(MASK, MASK);
	};
	x86_64::instructions::interrupts::enable();
}

const INIT: Option<Vec<fn(&InterruptStackFrame)>> = None;
static CALLBACKS: Mutex<[Option<Vec<fn(&InterruptStackFrame)>>; IRQS]> = Mutex::new([INIT; IRQS]);

fn irq_handler(stack_frame: InterruptStackFrame, irq: u8) {
	// serial_println!("Handling irq: {}", irq);
	match &CALLBACKS.lock()[irq as usize] {
		None => {}
		Some(vec) => {
			for callback in vec {
				callback(&stack_frame);
			}
		}
	}
	unsafe {
		PICS.lock().notify_end_of_interrupt(PIC_1_OFFSET + irq);
	}
}

/// Register a delegate function to be called when the interrupt with the given irq happens. You
/// can register multiple functions on the same irq, and they will be called in the order that they
/// are registered.
pub fn register_callback(irq: u8, callback: fn(&InterruptStackFrame)) {
	{
		x86_64::instructions::interrupts::disable();
		let callbacks = &mut CALLBACKS.lock();
		match &mut callbacks[irq as usize] {
			None => callbacks[irq as usize] = Some(vec![callback]),
			Some(vec) => vec.push(callback),
		}
	}
	x86_64::instructions::interrupts::enable();
}

extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
	exception("divide error", stack_frame)
}

extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
	exception("debug", stack_frame)
}

extern "x86-interrupt" fn non_maskable_interrupt_handler(stack_frame: InterruptStackFrame) {
	exception("non maskable interrupt", stack_frame)
}

/// Interrupt handler for breakpoint interrupts.
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
	exception("breakpoint", stack_frame)
}

extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
	exception("overflow", stack_frame)
}

extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: InterruptStackFrame) {
	exception("bound range exceeded", stack_frame)
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
	exception("invalid opcode", stack_frame)
}

extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
	exception("device not available", stack_frame)
}

/// Interrupt handler for double faults.
extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) -> ! {
	exception_error("double fault", stack_frame, error_code)
}

/// Interrupt handler for invalid tss.
extern "x86-interrupt" fn invalid_tss_handler(stack_frame: InterruptStackFrame, error_code: u64) {
	exception_error("invalid tss", stack_frame, error_code)
}

extern "x86-interrupt" fn segment_not_present_handler(stack_frame: InterruptStackFrame, error_code: u64) {
	exception_error("segment not present", stack_frame, error_code)
}

extern "x86-interrupt" fn stack_segment_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
	exception_error("stack segment fault", stack_frame, error_code)
}

extern "x86-interrupt" fn general_protection_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
	exception_error("general protection fault", stack_frame, error_code)
}

/// Interrupt handler for page faults. Currenty it **does not** solve the page fault (by swapping
/// pages, etc...), rather just prints some information about the fault.
extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
	use x86_64::registers::control::Cr2;
	serial_println!("EXCEPTION: page fault");
	serial_println!(" - Accessed Address: {:?}", Cr2::read());
	serial_println!(" - Error Code: {:?}", error_code);
	serial_println!(" - {:#?}", stack_frame);
	try_recover("page fault", stack_frame);
}

extern "x86-interrupt" fn x87_floating_point_handler(stack_frame: InterruptStackFrame) {
	exception("x87 floating point", stack_frame)
}

extern "x86-interrupt" fn alignment_check_handler(stack_frame: InterruptStackFrame, error_code: u64) {
	exception_error("alignment check", stack_frame, error_code)
}

extern "x86-interrupt" fn machine_check_handler(stack_frame: InterruptStackFrame) -> ! {
	exception("machine check", stack_frame)
}

extern "x86-interrupt" fn simd_floating_point_handler(stack_frame: InterruptStackFrame) {
	exception("simd floating point", stack_frame)
}

extern "x86-interrupt" fn virtualization_handler(stack_frame: InterruptStackFrame) {
	exception("virtualization", stack_frame)
}
extern "x86-interrupt" fn security_exception_handler(stack_frame: InterruptStackFrame, error_code: u64) {
	exception_error("security exception", stack_frame, error_code)
}

fn exception(string: &str, stack_frame: InterruptStackFrame) -> ! {
	serial_println!("EXCEPTION: {} \n - {:#?}", string, stack_frame);
	try_recover(string, stack_frame)
}

fn exception_error(string: &str, stack_frame: InterruptStackFrame, error_code: u64) -> ! {
	serial_println!(
		"EXCEPTION: {} \n - ERRORCODE:{} \n - {:#?}",
		string,
		error_code,
		stack_frame
	);
	try_recover(string, stack_frame)
}

fn try_recover(string: &str, stack_frame: InterruptStackFrame) -> ! {
	println!("\x1b[31m\x1b[4mEXCEPTION:\x1b[0m {}", string);
	use x86_64::{registers::segmentation::SegmentSelector, PrivilegeLevel};
	let code_selector = SegmentSelector(stack_frame.code_segment as u16);
	let from_userspace = code_selector.rpl() == PrivilegeLevel::Ring3;
	if from_userspace {
		process::remove_current_process();
	} else {
		loop {}
	}
}
