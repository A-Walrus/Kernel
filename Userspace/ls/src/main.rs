#![no_main]
#![no_std]

use standard::{get_args, println, syscalls::Dir};
extern crate alloc;
use alloc::string::String;

#[no_mangle]
pub extern "C" fn main() -> isize {
	let args = get_args();

	let path = args.get(0).unwrap_or(&"/");
	match Dir::open(path) {
		Ok(dir) => {
			let mut string = String::new();
			for entry in dir {
				string.push_str(&entry);
				string.push_str("  ");
			}
			println!("{}", string);
			return 0;
		}
		Err(_) => {
			println!("Failed to open directory: {}", path);
			return -1;
		}
	}
}
