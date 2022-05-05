#![no_main]
#![no_std]

use standard::{init, print, println, syscalls};
extern crate alloc;

#[no_mangle]
pub extern "C" fn _start() {
	init();

	let mut a: u64 = 0;
	let mut b: u64 = 1;

	let n = 100;

	// syscalls::print(&format!("First {} Fibonachi numbers:\n", n));
	println!("First {} Fibonachi numbers:", n);

	for _ in 0..n {
		print!("{}, ", a);
		b = a + b;
		a = b - a;
	}
	syscalls::exit(0);
}
