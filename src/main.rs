#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
extern crate alloc;

mod io;
mod memory;

use alloc::boxed::Box;
use io::serial;
use memory::allocator;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	let x = Box::new(5);
	loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
	serial_println!("{}", info);
	loop {}
}
