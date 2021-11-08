#![no_std]
#![no_main]

extern crate alloc;
use alloc::{boxed::Box, vec, vec::Vec};
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

	paging::map_heap(&boot_info.memory_regions);
	heap::setup();

	let x = Box::new(5);
	let mut vec = Vec::new();
	for i in 0..512 {
		vec.push(i)
	}
	serial_println!("{:?}", vec.as_slice());
	loop {}
}
