use crate::serial_println;
use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use pic8259::ChainedPics;
use spin;
use x86_64::structures::idt::PageFaultErrorCode;

/// Offset of the first pic in the chained pics
pub const PIC_1_OFFSET: u8 = 32;
/// Offset of the second pic in the chained pics
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

/// Mutex wrapping chained pics. This is the interface for communicating with the pics.
pub static PICS: spin::Mutex<ChainedPics> = spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Copy, Clone)]
#[repr(u8)]
/// Interrupt request type/id.
enum IRQ {
	Timer = 0,
	Keyboard = 1,
}

impl IRQ {
	/// Convert to [u8] index into IDT
	fn as_u8(&self) -> u8 {
		*self as u8 + PIC_1_OFFSET
	}

	/// Convert to [usize] index into IDT
	fn index(&self) -> usize {
		self.as_u8() as usize
	}
}

lazy_static! {
	static ref IDT: InterruptDescriptorTable = {
		let mut idt = InterruptDescriptorTable::new();
		idt.breakpoint.set_handler_fn(breakpoint_handler);
		idt.page_fault.set_handler_fn(page_fault_handler);
		idt[IRQ::Keyboard.index()].set_handler_fn(keyboard_interrupt_handler);
		idt[IRQ::Timer.index()].set_handler_fn(timer_interrupt_handler);
		idt
	};
}

/// Set up interrupt descriptor table, and chained pics
pub fn setup() {
	IDT.load();
	unsafe {
		let mut pics = PICS.lock();
		pics.initialize();
		const MASK: u8 = 0b1111_1100;
		pics.write_masks(MASK, MASK);
	};
	x86_64::instructions::interrupts::enable();
}

/// Interrupt handler for timer interrupts.
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
	unsafe {
		PICS.lock().notify_end_of_interrupt(IRQ::Timer.as_u8());
	}
}

/// Interupt handler for keyboard interrupts.
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
	use crate::io::keyboard;
	keyboard::read_input();
	unsafe {
		PICS.lock().notify_end_of_interrupt(IRQ::Keyboard.as_u8());
	}
}

/// Interrupt handler for breakpoint interrupts.
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
	serial_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
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
