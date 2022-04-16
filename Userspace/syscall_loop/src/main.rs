#![no_std]
#![no_main]
#![feature(asm)]

#[no_mangle]
pub extern "C" fn _start() -> ! {
	for _ in 0..100 {
		unsafe {
			asm!(
				"mov rax, 0x0", // sys debug
				"syscall",
			);
		}
	}
	unsafe {
		asm!(
			"mov rax, 0x1", // sys exit
			"syscall",
		);
	}
	loop {}
}

use core::panic::PanicInfo;
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
	loop {}
}
