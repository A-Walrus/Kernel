#![no_main]
#![no_std]
#![feature(asm)]

extern crate alloc;
use alloc::format;
#[allow(unused_imports)]
use standard::{io::*, syscalls::*, *};

#[no_mangle]
pub extern "C" fn main() -> isize {
	let mut file = File::create("/count.txt").unwrap();

	for i in 0..u64::MAX {
		println!("Count: {}", i);

		file.write(format!("{}, ", i).as_bytes()).unwrap();
	}
	return 0;
}
