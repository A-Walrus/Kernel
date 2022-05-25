#![no_std]
#![no_main]
#![feature(asm)]
#![feature(naked_functions)]

extern crate alloc;
use bootloader::{entry_point, BootInfo};
use core::time::Duration;
use kernel::{
	cpu::{gdt, interrupts, syscalls},
	fs::ext2,
	io::{buffer, keyboard},
	mem::{buddy, heap, paging},
	process, util,
};

entry_point!(kernel_main);

fn play_startup_song() {
	use util::play_note;
	util::wait_for_pit();
	let eigth_note = Duration::from_millis(230);
	let quarter_note = eigth_note * 2;
	let half_note = eigth_note * 4;
	let quarter_note_triplet = half_note / 3;
	let eigth_note_triplet = quarter_note / 3;
	let sixteenth_note_triplet = eigth_note / 3;

	// startup
	play_note(623, eigth_note + sixteenth_note_triplet);
	play_note(312, eigth_note_triplet);
	play_note(467, quarter_note);
	play_note(415, quarter_note + eigth_note_triplet);
	play_note(623, quarter_note_triplet);
	play_note(467, half_note);
}

/// Entry point for the kernel. Returns [!] because it is never supposed to exit.
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
		gdt::setup();
		paging::setup();
		buddy::setup(&boot_info.memory_regions);
		heap::setup(buffer::calc_real_length(framebuffer));
		interrupts::setup();
		syscalls::setup();
		keyboard::setup();
		util::setup();
		buffer::setup(framebuffer);

		ext2::setup().expect("Failed to setup EXT2");

		play_startup_song();

		for i in 0..buffer::TERM_COUNT {
			let s = alloc::format!("{}", i);
			process::add_process("/bin/shell", &[&s], Some(i)).expect("Failed to add process");
		}

		// process::add_process("/bin/pi", &["50000001"]).expect("Failed to add process");
		// process::add_process("/bin/pi", &["5000000"]).expect("Failed to add process");
		// process::add_process("/bin/b", &[]).expect("Failed to add process");

		process::start();
	}
	// kernel::end();
	loop {}
}
