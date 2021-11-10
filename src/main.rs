#![no_std]
#![no_main]

extern crate alloc;
use alloc::{boxed::Box, vec::Vec};
use bootloader::{entry_point, BootInfo};
use kernel::{
	cpu::{gdt, interrupts},
	mem::{heap, paging},
	serial_println,
};

entry_point!(kernel_main);

/// Entry point for the kernel. Returns [!] because it is never supposed to exit.
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	gdt::setup();
	interrupts::setup();

	heap::setup(&boot_info.memory_regions);

	let mut v = Vec::new();
	for i in 0..100000 {
		let a = Box::new(8);
		v.push(i);
	}
	serial_println!("Done");
	loop {}
}
