//! This crate is a WIP basic OS kernel written in rust.
//! This OS targets x86 64 bit computers.
//! It uses the [Bootloader Crate] as its bootloader.
//!
//! [Bootloader Crate]: https://crates.io/crates/bootloader

#![no_std] // No standard library
#![feature(abi_x86_interrupt)] // x86 interrupts
#![feature(const_for)] // for loops in const functions
#![feature(const_mut_refs)] // mutable references inside const functions
#![feature(alloc_error_handler)] // error handler for alloc failiures
#![feature(int_log)] // log2 for ints (using single assembly instruction to find highest bit)
#![feature(slice_ptr_get)]
#![feature(stmt_expr_attributes)]
#![feature(asm)] // inline asm
#![feature(slice_ptr_len)]
#![feature(naked_functions)] // naked functions (no prologue and epilogue)
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

#[macro_use]
extern crate alloc;

/// Dealing with cpu related structures and registers: IDT, GDT, TLB...
pub mod cpu;

/// Dealing with input/output: screen, keyboard, serial...
#[macro_use]
pub mod io;

/// Dealing with memory: paging, tlb, alloc/heap...
pub mod mem;

/// Dealing with drivers
pub mod drivers;

/// Dealing with filesystems and partitions
pub mod fs;

/// Utilities
pub mod util;

/// Module for dealing with ELF executables
pub mod process;

use core::panic::PanicInfo;

const SOUND_ENABLE: bool = false;

/// End the os
pub fn end() -> ! {
	fs::ext2::cleanup().expect("Failed to cleanup EXT2");
	serial_println!("Finished cleanup");

	unsafe {
		process::RUNNING = false;
	}
	x86_64::instructions::interrupts::enable();

	println!("Shutting down...");

	cpu::pit::play_shutdown_song();

	util::qemu::exit();
}

/// Panic handler is called automatically when a panic occurs, and prints the information to serial
/// for debugging.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
	serial_println!("{}", info);
	loop {}
}
