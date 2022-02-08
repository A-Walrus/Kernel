#![no_std]
#![no_main]

extern crate alloc;
use bootloader::{entry_point, BootInfo};
use kernel::{
	cpu::{gdt, interrupts},
	drivers,
	io::buffer,
	mem::{buddy, heap, paging},
	println, serial_println,
};

entry_point!(kernel_main);

/// Entry point for the kernel. Returns [!] because it is never supposed to exit.
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
		gdt::setup();
		paging::setup();
		buddy::setup(&boot_info.memory_regions);

		let len = buffer::calc_real_length(framebuffer);
		heap::setup(len);
		interrupts::setup();
		serial_println!("Setup complete!");

		let screen = buffer::Screen::new_from_framebuffer(framebuffer);

		let term = buffer::Terminal::new(screen);

		unsafe {
			buffer::TERM = Some(term);
		}
		println!("Free RAM {} MiB", buddy::ALLOCATOR.lock().get_free_space() >> 20);

		// drivers::pci::testing();
		unsafe {
			drivers::ahci::setup();
		}
	}
	loop {}
}
