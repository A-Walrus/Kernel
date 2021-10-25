use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use crate::{serial_print, serial_println};
use pic8259::ChainedPics;
use spin;
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> = spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Copy, Clone)]
#[repr(u8)]
enum IRQ {
	Timer = 0,
	Keyboard = 1,
}

impl IRQ {
	fn as_u8(&self) -> u8 {
		*self as u8 + PIC_1_OFFSET
	}

	fn index(&self) -> usize {
		self.as_u8() as usize
	}
}

lazy_static! {
	static ref IDT: InterruptDescriptorTable = {
		let mut idt = InterruptDescriptorTable::new();
		idt.breakpoint.set_handler_fn(breakpoint_handler);
		idt[IRQ::Keyboard.index()].set_handler_fn(keyboard_interrupt_handler);
		idt[IRQ::Timer.index()].set_handler_fn(timer_interrupt_handler);
		idt
	};
}

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

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
	unsafe {
		PICS.lock().notify_end_of_interrupt(IRQ::Timer.as_u8());
	}
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
	use crate::io::keyboard;
	keyboard::read_input();
	unsafe {
		PICS.lock().notify_end_of_interrupt(IRQ::Keyboard.as_u8());
	}
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
	serial_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}
