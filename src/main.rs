#![no_std]
#![no_main]

use bootloader::{entry_point, BootInfo};
use core::{mem::transmute, panic::PanicInfo};

mod io;
use io::buffer::Pixel;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
		let buffer = framebuffer.buffer_mut();
		// let back: &[Pixel] = &[Pixel {
		// 	red: 0xff,
		// 	green: 0,
		// 	blue: 0,
		// 	unknown: 0xff,
		// }; 480256];
		// unsafe {
		// 	let front = buffer as *mut [u8];
		// 	let front = front as *mut [Pixel];
		// 	let front = &mut *front;
		// 	front.copy_from_slice(back);
		// }
	}

	loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	loop {}
}
