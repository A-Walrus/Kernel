#![no_main]
#![no_std]

use alloc::{format, vec::Vec};
use standard::{
	init,
	io::Write,
	print, println,
	syscalls::{self, File},
};
extern crate alloc;

#[no_mangle]
pub extern "C" fn main() -> isize {
	let mut a: u64 = 0;
	let mut b: u64 = 1;

	let n = 100;

	// syscalls::print(&format!("First {} Fibonachi numbers:\n", n));
	println!("First {} Fibonachi numbers:", n);

	let mut v = Vec::new();
	for _ in 0..n {
		v.push(a);
		print!("{}, ", a);
		b = a + b;
		a = b - a;
	}
	let string = format!("{:?}", v);

	let mut file = File::new("/test.txt").unwrap();
	file.write(string.as_bytes()).unwrap();

	return 0;
}
