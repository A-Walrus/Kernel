#![no_std]
#![no_main]
#![feature(asm)]

extern crate alloc;
use bootloader::{entry_point, BootInfo};
use kernel::{
	cpu::{gdt, interrupts},
	fs::ext2,
	io::buffer,
	mem::{buddy, heap, paging},
	serial_println,
};

entry_point!(kernel_main);

fn test_function() {
	// serial_println!("Test function");
	loop {}
}

/// Entry point for the kernel. Returns [!] because it is never supposed to exit.
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
		gdt::setup();
		paging::setup();
		buddy::setup(&boot_info.memory_regions);
		heap::setup(buffer::calc_real_length(framebuffer));
		interrupts::setup();
		serial_println!("Setup complete!");

		let screen = buffer::Screen::new_from_framebuffer(framebuffer);
		let term = buffer::Terminal::new(screen);
		unsafe {
			buffer::TERM = Some(term);
		}
		// ext2::setup().expect("Failed to setup EXT2");
		serial_println!("Finished setup");

		// TODO do stuff here

		unsafe {
			// sysret method
			// Taken from osdev, changed ecx, to rcx since ecx is 32 bits, not 64
			asm!(
				"mov rcx, 0xc0000082",
				"wrmsr",
				"mov rcx, 0xc0000080",
				"wrmsr",
				"or eax, 1",
				"wrmsr",
				"mov rcx, 0xc0000081",
				"rdmsr",
				"mov edx, 0x00180008",
				"wrmsr",
				"mov rcx, {function}",
				"mov r11, 0x202",
				"sysretq",
				function = in(reg) test_function,
			);

			// iret method (doesn't compile because 64 bit mode)
			// asm!(
			// 	"mov ax, (4 * 8) | 3",
			// 	"mov ds, ax",
			// 	"mov es, ax",
			// 	"mov fs, ax",
			// 	"mov gs, ax",
			// 	"mov eax, esp",
			// 	"push (4 * 8) | 3",
			// 	"push eax",
			// 	"pushf",
			// 	"push (3 * 8) | 3",
			// 	"push {function}",
			// 	"iret",
			// 	function = in(reg) test_function,
			// );

			// // Testing jumping, and function pointers. Seems to work
			// asm!(
			// 	"jmp {function}",
			// 	function = in(reg) test_function,
			// );
		}
		serial_println!("After inline asm");

		// ext2::cleanup().expect("Failed to cleanup EXT2");
		serial_println!("Finished cleanup");
	}
	serial_println!("The end");
	loop {}
}
