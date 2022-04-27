#![no_std]
#![no_main]
#![feature(asm)]
#![feature(naked_functions)]

extern crate alloc;
use bootloader::{entry_point, BootInfo};
use kernel::{
	cpu::{gdt, interrupts, syscalls},
	fs::ext2,
	io::buffer,
	mem::{buddy, heap, paging},
	process, serial_println,
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
		let screen = buffer::Screen::new_from_framebuffer(framebuffer);
		let term = buffer::Terminal::new(screen);
		unsafe {
			buffer::TERM = Some(term);
		}
		ext2::setup().expect("Failed to setup EXT2");
		serial_println!("Finished setup");

		process::add_process("/bin/a").expect("Failed to add process");
		process::add_process("/bin/b").expect("Failed to add process");

		process::run_next_process();

		ext2::cleanup().expect("Failed to cleanup EXT2");
		serial_println!("Finished cleanup");
	}
	serial_println!("The end");
	loop {
		x86_64::instructions::hlt()
	}
}
