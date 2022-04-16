use super::{irq_handler, PIC_1_OFFSET};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

pub fn set_irq_handlers(idt: &mut InterruptDescriptorTable) {
	unsafe {
		idt[(PIC_1_OFFSET + 0) as usize].set_handler_fn(irq_handler_0);
		idt[(PIC_1_OFFSET + 1) as usize]
			.set_handler_fn(irq_handler_1)
			.set_stack_index(0);
		idt[(PIC_1_OFFSET + 2) as usize].set_handler_fn(irq_handler_2);
		idt[(PIC_1_OFFSET + 3) as usize].set_handler_fn(irq_handler_3);
		idt[(PIC_1_OFFSET + 4) as usize].set_handler_fn(irq_handler_4);
		idt[(PIC_1_OFFSET + 5) as usize].set_handler_fn(irq_handler_5);
		idt[(PIC_1_OFFSET + 6) as usize].set_handler_fn(irq_handler_6);
		idt[(PIC_1_OFFSET + 7) as usize].set_handler_fn(irq_handler_7);
		idt[(PIC_1_OFFSET + 8) as usize].set_handler_fn(irq_handler_8);
		idt[(PIC_1_OFFSET + 9) as usize].set_handler_fn(irq_handler_9);
		idt[(PIC_1_OFFSET + 10) as usize].set_handler_fn(irq_handler_10);
		idt[(PIC_1_OFFSET + 11) as usize].set_handler_fn(irq_handler_11);
		idt[(PIC_1_OFFSET + 12) as usize].set_handler_fn(irq_handler_12);
		idt[(PIC_1_OFFSET + 13) as usize].set_handler_fn(irq_handler_13);
		idt[(PIC_1_OFFSET + 14) as usize].set_handler_fn(irq_handler_14);
		idt[(PIC_1_OFFSET + 15) as usize].set_handler_fn(irq_handler_15);
	}
}

extern "x86-interrupt" fn irq_handler_0(stack_frame: InterruptStackFrame) {
	irq_handler(stack_frame, 0);
}

extern "x86-interrupt" fn irq_handler_1(stack_frame: InterruptStackFrame) {
	irq_handler(stack_frame, 1);
}

extern "x86-interrupt" fn irq_handler_2(stack_frame: InterruptStackFrame) {
	irq_handler(stack_frame, 2);
}

extern "x86-interrupt" fn irq_handler_3(stack_frame: InterruptStackFrame) {
	irq_handler(stack_frame, 3);
}

extern "x86-interrupt" fn irq_handler_4(stack_frame: InterruptStackFrame) {
	irq_handler(stack_frame, 4);
}

extern "x86-interrupt" fn irq_handler_5(stack_frame: InterruptStackFrame) {
	irq_handler(stack_frame, 5);
}

extern "x86-interrupt" fn irq_handler_6(stack_frame: InterruptStackFrame) {
	irq_handler(stack_frame, 6);
}

extern "x86-interrupt" fn irq_handler_7(stack_frame: InterruptStackFrame) {
	irq_handler(stack_frame, 7);
}

extern "x86-interrupt" fn irq_handler_8(stack_frame: InterruptStackFrame) {
	irq_handler(stack_frame, 8);
}

extern "x86-interrupt" fn irq_handler_9(stack_frame: InterruptStackFrame) {
	irq_handler(stack_frame, 9);
}

extern "x86-interrupt" fn irq_handler_10(stack_frame: InterruptStackFrame) {
	irq_handler(stack_frame, 10);
}

extern "x86-interrupt" fn irq_handler_11(stack_frame: InterruptStackFrame) {
	irq_handler(stack_frame, 11);
}

extern "x86-interrupt" fn irq_handler_12(stack_frame: InterruptStackFrame) {
	irq_handler(stack_frame, 12);
}

extern "x86-interrupt" fn irq_handler_13(stack_frame: InterruptStackFrame) {
	irq_handler(stack_frame, 13);
}

extern "x86-interrupt" fn irq_handler_14(stack_frame: InterruptStackFrame) {
	irq_handler(stack_frame, 14);
}

extern "x86-interrupt" fn irq_handler_15(stack_frame: InterruptStackFrame) {
	irq_handler(stack_frame, 15);
}
