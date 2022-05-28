#![no_main]
#![no_std]

use standard::{
	io::{Read, Write},
	syscalls::*,
	*,
};
extern crate alloc;
use alloc::vec::Vec;

#[no_mangle]
pub extern "C" fn main() -> isize {
	let args = get_args();

	if args.len() == 2 {
		let source = args[0];
		let dest = args[1];

		let mut source = File::open(source).unwrap();
		let mut dest = File::open(dest).unwrap();

		let mut buf = Vec::new();
		source.read_to_end(&mut buf).unwrap();
		let mut trash = Vec::new();
		dest.read_to_end(&mut trash).unwrap();
		dest.write(&buf).unwrap();

		return 0;
	} else {
		println!("Incorrect numberr of args");
		return -1;
	}
}
