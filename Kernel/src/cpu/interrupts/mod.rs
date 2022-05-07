use crate::serial_println;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::structures::idt::PageFaultErrorCode;

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
			idt.divide_error.set_handler_fn(divide_error_handler);
			idt.debug.set_handler_fn(debug_handler);
			idt.non_maskable_interrupt.set_handler_fn(non_maskable_interrupt_handler);
			idt.breakpoint.set_handler_fn(breakpoint_handler);
			idt.overflow.set_handler_fn(overflow_handler);
			idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler);
			idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
			idt.device_not_available.set_handler_fn(device_not_available_handler);
			idt.double_fault.set_handler_fn(double_fault_handler).set_stack_index(0);
			idt.invalid_tss.set_handler_fn(invalid_tss_handler);
			idt.segment_not_present.set_handler_fn(segment_not_present_handler);
			idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
			idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
			idt.page_fault.set_handler_fn(page_fault_handler);
			idt.x87_floating_point.set_handler_fn(x87_floating_point_handler);
			idt.alignment_check.set_handler_fn(alignment_check_handler);
			idt.machine_check.set_handler_fn(machine_check_handler);
			idt.simd_floating_point.set_handler_fn(simd_floating_point_handler);
			idt.virtualization.set_handler_fn(virtualization_handler);
			idt.security_exception.set_handler_fn(security_exception_handler);
		}


		set_irq_handlers(&mut idt);

		// register_callback(0,timer_interrupt_handler);
		// register_callback(1,keyboard_interrupt_handler);

		Mutex::new(idt)
	};
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
	let callbacks = &mut CALLBACKS.lock();
	match &mut callbacks[irq as usize] {
		None => callbacks[irq as usize] = Some(vec![callback]),
		Some(vec) => vec.push(callback),
	}
}

fn timer_interrupt_handler(_stack_frame: &InterruptStackFrame) {
	// print!(".");
}

// fn keyboard_interrupt_handler(_stack_frame: &InterruptStackFrame) {
// use crate::io::keyboard;
// keyboard::read_input();
// }

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
	loop {}
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
	loop {}
}

fn exception_error(string: &str, stack_frame: InterruptStackFrame, error_code: u64) -> ! {
	serial_println!(
		"EXCEPTION: {} \n - ERRORCODE:{} \n - {:#?}",
		string,
		error_code,
		stack_frame
	);
	loop {}
}
