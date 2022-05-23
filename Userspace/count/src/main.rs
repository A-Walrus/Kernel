#![no_main]
#![no_std]
#![feature(asm)]

extern crate alloc;
#[allow(unused_imports)]
use standard::{syscalls::*, *};

#[no_mangle]
pub extern "C" fn main() -> isize {
	for i in 0..u64::MAX {
		println!("Count: {}", i);
	}
	return 0;
}
