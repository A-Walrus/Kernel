#![no_std]
#![no_main]

use bootloader::{entry_point, BootInfo};
use core::{mem::transmute, panic::PanicInfo};

mod io;
use io::{buffer::Pixel, serial};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
		let buffer = framebuffer.buffer_mut();
		let back: &[Pixel] = &[Pixel {
			red: 0x0,
			green: 0x0,
			blue: 0xFF,
			padding: 0xff,
		}; 480256];
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
