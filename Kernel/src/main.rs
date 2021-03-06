#![no_std]
#![no_main]
#![feature(asm)]
#![feature(naked_functions)]

extern crate alloc;
use bootloader::{entry_point, BootInfo};
use kernel::{
	cpu::{gdt, interrupts, pit, syscalls},
	fs::ext2,
	io::{buffer, keyboard},
	mem::{buddy, heap, paging},
	process,
};

entry_point!(kernel_main);

/// Entry point for the kernel. Returns [!] because it is never supposed to exit.
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
		gdt::setup();
		paging::setup();
		buddy::setup(&boot_info.memory_regions);
		heap::setup(buffer::calc_real_length(framebuffer));
		interrupts::setup();
		syscalls::setup();
		pit::setup_time();
		keyboard::setup();
		buffer::setup(framebuffer);

		ext2::setup().expect("Failed to setup EXT2");

		pit::play_startup_song();

		for i in 0..buffer::TERM_COUNT {
			let s = alloc::format!("{}", i);
			process::add_process("/bin/shell", &[&s], Some(i)).expect("Failed to add process");
		}

		process::start();
	}
	// kernel::end();
	loop {}
}
