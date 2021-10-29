//! This crate is a WIP basic OS kernel written in rust.
//! This OS targets x86 64 bit computers.
//! It uses the [Bootloader Crate] as its bootloader.
//!
//! [Bootloader Crate]: https://crates.io/crates/bootloader

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

mod gdt;
mod interrupts;
mod io;
use io::serial;

#[cfg(not(doc))]
entry_point!(kernel_main);

/// Entry point for the kernel. Returns [!] because it is never supposed to exit.
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	gdt::setup();
	interrupts::setup();
	loop {}
}

/// Panic handler is called automatically when a panic occurs, and prints the information to serial
/// for debugging
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
	serial_println!("{}", info);
	loop {}
}
