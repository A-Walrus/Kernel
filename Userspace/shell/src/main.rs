#![no_std]
#![no_main]
#![feature(asm)]

pub mod syscalls;
use syscalls::*;

#[no_mangle]
pub extern "C" fn _start() -> ! {
	loop {
		print("> ");
	}
}

use core::panic::PanicInfo;
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
	loop {}
}
