#![no_main]
#![no_std]

use standard::{init, syscalls};
extern crate alloc;

use alloc::{format, string::String};

#[no_mangle]
pub extern "C" fn _start() {
	init();

	let mut a: u64 = 0;
	let mut b: u64 = 1;

	let n = 100;

	syscalls::print(&format!("First {} Fibonachi numbers:\n", n));

	for _ in 0..n {
		let s: String = format!("{}, ", a);
		b = a + b;
		a = b - a;
		syscalls::print(&s);
	}
	syscalls::exit(0);
}
