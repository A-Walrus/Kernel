#![no_std]
#![no_main]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

mod io;
use io::{buffer::Pixel, serial};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
		let buffer = framebuffer.buffer_mut();
		let pixel = Pixel::new(0, 0, 0xff);
		let back: &[Pixel] = &[pixel; 480256];
		unsafe {
			let front = buffer as *mut [u8];
			let front = front as *mut [Pixel; 480256];
			let front = &mut *front;
			front.copy_from_slice(back);
		}
	}

	loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
	serial_println!("{}", info);
	loop {}
}
