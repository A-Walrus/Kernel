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
				println!("Bytes: {}", buf.len());
				let res = String::from_utf8(buf);
				match res {
					Ok(string) => {
						let word_count = string.split_whitespace().count();
						let line_count = string.lines().count();
						println!("Words: {}", word_count);
						println!("Lines: {}", line_count);
					}
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
