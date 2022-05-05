#![no_main]
#![no_std]

use standard::{init, println, syscalls};
extern crate alloc;

#[no_mangle]
pub extern "C" fn _start() {
	init();

	for i in 0..100 {
		println!("number {}", i);
	}
	syscalls::exit(0);
}
