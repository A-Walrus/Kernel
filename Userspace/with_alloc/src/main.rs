#![no_main]
#![no_std]

use standard::{init, syscalls};
extern crate alloc;

use alloc::{format, string::String};

#[no_mangle]
pub extern "C" fn _start() {
	init();

	for i in 0..100 {
		let s: String = format!("number {}\n", i);
		syscalls::print(&s);
	}
	syscalls::exit(0);
}
