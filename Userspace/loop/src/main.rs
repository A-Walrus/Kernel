#![no_main]
#![no_std]
#![feature(asm)]

extern crate alloc;
#[allow(unused_imports)]
use standard;

#[no_mangle]
pub extern "C" fn main() -> isize {
	loop {}
	// return 0;
}
