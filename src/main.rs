#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

mod gdt;
mod interrupts;
mod io;
mod mem;
use io::serial;
use mem::paging;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	gdt::setup();
	interrupts::setup();
	let top_level = unsafe { paging::get_current_page_table() };
	paging::print_table_recursive(top_level, 4);
	loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
	serial_println!("{}", info);
	loop {}
}
