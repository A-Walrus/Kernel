use crate::{print, serial_println};
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
		idt.breakpoint.set_handler_fn(breakpoint_handler);
		idt.page_fault.set_handler_fn(page_fault_handler);
		// idt[IRQ::Keyboard.index()].set_handler_fn(keyboard_interrupt_handler);
		// idt[IRQ::Timer.index()].set_handler_fn(timer_interrupt_handler);
		idt.double_fault.set_handler_fn(double_fault_handler);
		idt.invalid_tss.set_handler_fn(invalid_tss_handler);
		set_irq_handlers(&mut idt);

		register_callback(0,timer_interrupt_handler);
		register_callback(1,keyboard_interrupt_handler);

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
	print!(".");
}
fn keyboard_interrupt_handler(_stack_frame: &InterruptStackFrame) {
	use crate::io::keyboard;
	keyboard::read_input();
}

// /// Interrupt handler for timer interrupts.
// extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
// 	unsafe {
// 		PICS.lock().notify_end_of_interrupt(IRQ::Timer.as_u8());
// 	}
// }

// /// Interupt handler for keyboard interrupts.
// extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
// 	use crate::io::keyboard;
// 	keyboard::read_input();
// 	unsafe {
// 		PICS.lock().notify_end_of_interrupt(IRQ::Keyboard.as_u8());
// 	}
// }

/// Interrupt handler for breakpoint interrupts.
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
	serial_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

/// Interrupt handler for double faults.
extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) -> ! {
	serial_println!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
	loop {}
}

/// Interrupt handler for invalid tss.
extern "x86-interrupt" fn invalid_tss_handler(stack_frame: InterruptStackFrame, error_code: u64) {
	serial_println!("EXCEPTION: INVALID TSS\n{:#?}", stack_frame);
	loop {}
}

/// Interrupt handler for page faults. Currenty it **does not** solve the page fault (by swapping
/// pages, etc...), rather just prints some information about the fault.
extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
	use x86_64::registers::control::Cr2;

	serial_println!("EXCEPTION: PAGE FAULT");
	serial_println!("Accessed Address: {:?}", Cr2::read());
	serial_println!("Error Code: {:?}", error_code);
	serial_println!("{:#?}", stack_frame);
	loop {}
}
