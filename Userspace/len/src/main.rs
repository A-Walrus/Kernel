#![no_main]
#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use standard::{get_args, io::Read, println, syscalls::File};

#[no_mangle]
pub extern "C" fn main() -> isize {
	let args = get_args();

	for path in args {
		let file = File::open(path);
		match file {
			Ok(mut f) => {
				let mut buf = Vec::new();
				f.read_to_end(&mut buf).expect("Failed to read!");
				println!("File length: {}", buf.len());
			}
			Err(_) => {
				println!("Failed to open file")
			}
		}
	}

	return 0;
}
