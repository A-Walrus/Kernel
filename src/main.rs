#![no_std]
#![no_main]

extern crate alloc;
use alloc::boxed::Box;
use bootloader::{entry_point, BootInfo};
use kernel::{
	cpu::{gdt, interrupts},
	mem::{heap, paging},
};

entry_point!(kernel_main);

/// Entry point for the kernel. Returns [!] because it is never supposed to exit.
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	gdt::setup();
	interrupts::setup();
	let top_level = paging::get_current_page_table();
	//paging::print_table_recursive(top_level, 4);
	let x = Box::new(5);
	loop {}
}
