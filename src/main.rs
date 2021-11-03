#![no_std]
#![no_main]

use bootloader::{entry_point, BootInfo};
use kernel::{
	cpu::{gdt, interrupts},
	mem::paging,
};

entry_point!(kernel_main);

/// Entry point for the kernel. Returns [!] because it is never supposed to exit.
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	gdt::setup();
	interrupts::setup();
	let top_level = unsafe { paging::get_current_page_table() };
	//paging::print_table_recursive(top_level, 4);
	loop {}
}
