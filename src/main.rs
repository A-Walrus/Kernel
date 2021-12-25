#![no_std]
#![no_main]

extern crate alloc;
use alloc::{boxed::Box, vec, vec::Vec};
use bootloader::{entry_point, BootInfo};
use kernel::{
	cpu::{gdt, interrupts},
	io::buffer,
	mem::{buddy, heap, paging},
	serial_println,
};

entry_point!(kernel_main);

/// Entry point for the kernel. Returns [!] because it is never supposed to exit.
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	gdt::setup();
	interrupts::setup();
	paging::setup();
	buddy::setup(&boot_info.memory_regions);
	heap::setup();
	serial_println!("Setup complete!");

	if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
		let mut screen = buffer::Screen::new_from_framebuffer(framebuffer);

		let mut terminal = buffer::Terminal::new(screen);
		terminal.write("Hello World");
		terminal.redraw();
	}
	loop {}
}
