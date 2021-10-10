#![no_std]
#![no_main]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	// turn the screen gray
	if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
		let other_buffer: &[u8] = &[0xff; 1921024];

		framebuffer.buffer_mut().copy_from_slice(other_buffer);
	}

	loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	loop {}
}
