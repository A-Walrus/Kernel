#![no_std]
#![no_main]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

mod gdt;
mod io;
use crate::buffer::SCREEN_SIZE;
use io::{
	buffer::{self, Pixel, Screen, Terminal},
	serial,
};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	gdt::setup();
	loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
	serial_println!("{}", info);
	loop {}
}
