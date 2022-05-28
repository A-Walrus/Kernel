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

		match File::open(source) {
			Ok(mut source_file) => {
				let mut buf = Vec::new();
				source_file.read_to_end(&mut buf).expect("Failed to read!");

				match File::create(dest) {
					Ok(mut dest_file) => {
						dest_file.write(&buf).expect("Failed to write!");
						drop(source_file);
						unlink(source).expect("Failed to delte source!");
						return 0;
					}
					Err(_) => {
						println!("Failed to create file!");
						return 0;
					}
				}
			}
			Err(_) => {
				println!("Failed to open file!");
				return 0;
			}
		}
	} else {
		println!("Incorrect numberr of args");
		return -1;
	}
}
