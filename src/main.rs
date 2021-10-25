#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use io::buffer::{self, Pixel, Screen, Terminal, SCREEN_SIZE};
mod gdt;
mod interrupts;
mod io;
use io::serial;

entry_point!(kernel_main);

static mut BACK: &mut [Pixel] = &mut [Pixel::new(0, 0, 0); SCREEN_SIZE];
static mut TERMINAL: Option<Terminal> = None;

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
		let buffer = framebuffer.buffer_mut();
		unsafe {
			let mut screen = Screen::new(as_pixels!(framebuffer.buffer_mut()), BACK, framebuffer.info());

			let mut terminal = Terminal::new(screen);
			TERMINAL = Some(terminal);
		}
	}
	gdt::setup();
	interrupts::setup();

	loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
	serial_println!("{}", info);
	loop {}
}
