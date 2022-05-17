#![no_main]
#![no_std]

use standard::{get_args, println, syscalls::Dir};
extern crate alloc;

#[no_mangle]
pub extern "C" fn main() -> isize {
	let args = get_args();

	let path = args.get(0).unwrap_or(&"/");
	let dir = Dir::open(path).unwrap();
	for entry in dir {
		println!("{}", entry);
	}
	return 0;
}
