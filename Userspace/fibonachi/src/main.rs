#![no_main]
#![no_std]

use alloc::{format, vec::Vec};
use standard::{get_args, io::Write, print, println, syscalls::File};
extern crate alloc;

#[no_mangle]
pub extern "C" fn main() -> isize {
	let args = get_args();

	let n: usize = match args.get(0).map(|s| s.parse()) {
		None => 10,
		Some(Ok(num)) => num,
		Some(Err(_)) => {
			println!("Arguement must be a number!");
			return -1;
		}
	};

	let mut a: u64 = 0;
	let mut b: u64 = 1;

	println!("First {} Fibonachi numbers:", n);

	let mut v = Vec::new();
	for _ in 0..n {
		v.push(a);
		print!("{}, ", a);
		b = a + b;
		a = b - a;
	}
	let string = format!("{:?}", v);

	let mut file = File::create("/fib.txt").unwrap();
	file.write(string.as_bytes()).unwrap();

	return 0;
}
