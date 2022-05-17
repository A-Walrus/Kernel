#![no_main]
#![no_std]

extern crate alloc;
use alloc::{string::String, vec::Vec};
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
				let res = String::from_utf8(buf);
				match res {
					Ok(string) => println!("{}", string),
					Err(_) => println!("File is not UTF8"),
				}
			}
			Err(_) => {
				println!("Failed to open file")
			}
		}
	}

	return 0;
}
