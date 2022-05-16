#![no_main]
#![no_std]

extern crate alloc;
use alloc::{string::String, vec::Vec};
use standard::{get_args, io::Read, println, syscalls::File};

#[no_mangle]
pub extern "C" fn main() -> isize {
	let args = get_args();
	let message = args.get(0).unwrap_or(&"");
	println!("{}", message);

	return 0;
}
