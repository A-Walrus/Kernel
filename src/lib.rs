//! This crate is a WIP basic OS kernel written in rust.
//! This OS targets x86 64 bit computers.
//! It uses the [Bootloader Crate] as its bootloader.
//!
//! [Bootloader Crate]: https://crates.io/crates/bootloader

#![no_std]
#![feature(abi_x86_interrupt)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![feature(alloc_error_handler)]

/// Dealing with cpu related structures and registers: IDT, GDT, TLB...
pub mod cpu;

/// Dealing with input/output: screen, keyboard, serial...
pub mod io;

/// Dealing with memory: paging, tlb, alloc/heap...
pub mod mem;

use core::panic::PanicInfo;

pub use io::serial;

/// Panic handler is called automatically when a panic occurs, and prints the information to serial
/// for debugging.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
	serial_println!("{}", info);
	loop {}
}
