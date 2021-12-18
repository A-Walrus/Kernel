#![no_std]
#![no_main]

extern crate alloc;
use alloc::{boxed::Box, vec, vec::Vec};
use bootloader::{entry_point, BootInfo};
use kernel::{
	cpu::{gdt, interrupts},
	mem::{buddy, heap, paging},
	serial_println,
};

entry_point!(kernel_main);

/// Entry point for the kernel. Returns [!] because it is never supposed to exit.
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	gdt::setup();
	interrupts::setup();
	paging::setup();
	serial_println!("About to setup buddy allocator!");
	buddy::setup(&boot_info.memory_regions);
	serial_println!("Free RAM: {}", buddy::ALLOCATOR.lock().get_free_space());
	heap::setup();
	serial_println!("Setup complete!");

	let x = Box::new(5);
	serial_println!("Made box");
	let mut vec = Vec::new();
	serial_println!("Made vec");
	for i in 0..512 {
		vec.push(i)
	}
	serial_println!("{:?}", x);
	serial_println!("{:?}", vec.as_slice());
	loop {}
}
