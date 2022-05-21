#![no_main]
#![no_std]
#![feature(asm)]

extern crate alloc;
use standard::{get_args, println};

#[no_mangle]
pub extern "C" fn main() -> isize {
	for i in 0..u64::MAX {
		println!("Count: {}", i);
	}
	return 0;
}
