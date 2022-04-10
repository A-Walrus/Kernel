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
#![feature(asm)]
#![feature(slice_ptr_len)]
#![feature(naked_functions)]
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
pub mod elf;

use core::panic::PanicInfo;

/// Panic handler is called automatically when a panic occurs, and prints the information to serial
/// for debugging.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
	serial_println!("{}", info);
	loop {}
}
