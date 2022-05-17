#![no_main]
#![no_std]

extern crate alloc;
use standard::{get_args, println, syscalls::File};

#[no_mangle]
pub extern "C" fn main() -> isize {
	let args = get_args();
	match args.get(0) {
		Some(path) => {
			File::create(path).expect("Failed to create/open file");
		}
		None => {
			println!("missing file arguement")
		}
	}

	return 0;
}
