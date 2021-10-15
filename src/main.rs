#![no_std]
#![no_main]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

mod io;
use crate::buffer::SCREEN_SIZE;
use io::{
	buffer::{self, Pixel, PixelPos, Screen, TextBuffer},
	serial,
};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
		let buffer = framebuffer.buffer_mut();
		let pixel = Pixel::new(0, 0, 0x0);
		let back: &mut [Pixel] = &mut [pixel; buffer::SCREEN_SIZE];
		let mut screen = Screen::new(as_pixels!(framebuffer.buffer_mut()), back, framebuffer.info());
		let mut text_buf = TextBuffer { screen };

		const WIDTH: usize = 64;
		for i in 0..255 {
			let x = i % WIDTH;
			let y = i / WIDTH;
			text_buf.draw_char(i, PixelPos::new(x * 8, y * 16), Pixel::new(255, 255, 255));
		}
		text_buf.screen.flush();
	}

	loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
	serial_println!("{}", info);
	loop {}
}
