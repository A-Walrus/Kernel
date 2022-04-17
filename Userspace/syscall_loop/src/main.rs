#![no_std]
#![no_main]
#![feature(asm)]

#[no_mangle]
pub extern "C" fn _start() -> ! {
	let s = "Hello from Userland!\n";
	let addr = s.as_ptr();
	let len = s.len();
	for _ in 0..10 {
		unsafe {
			asm!(
				"mov rax, 0x1", // sys print
				"syscall",
				in("rdi") addr,
				in("rsi") len
			);
		}
	}
	loop {}
}

use core::panic::PanicInfo;
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
	loop {}
}
