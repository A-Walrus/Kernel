#![no_main]
#![no_std]

use standard::{syscalls::*, *};
extern crate alloc;

#[no_mangle]
pub extern "C" fn main() -> isize {
	let args = get_args();

	if let Some(path) = args.get(0) {
		match rmdir(path) {
			Ok(_) => return 0,
			Err(()) => return -1,
		}
	} else {
		println!("One arguement required!");
		return -1;
	}
}
