#![no_main]
#![no_std]

use standard::{
	get_args, println,
	syscalls::{unlink, Dir},
};
extern crate alloc;
use alloc::string::String;

#[no_mangle]
pub extern "C" fn main() -> isize {
	let args = get_args();

	let path = args.get(0);
	if let Some(path) = args.get(0) {
		match unlink(path) {
			Ok(_) => return 0,
			Err(()) => return -1,
		}
	} else {
		println!("One arguement required!");
		return -1;
	}
}
