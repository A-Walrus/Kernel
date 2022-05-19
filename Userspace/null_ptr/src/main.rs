#![no_main]
#![no_std]
#![feature(asm)]

extern crate alloc;
#[allow(unused_imports)] // import is actually used...
use standard;

#[no_mangle]
pub extern "C" fn main() -> isize {
	unsafe { asm!("mov [0], {a}",a = in(reg) 5u64 ) }
	return 0;
}
