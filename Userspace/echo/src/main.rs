#![no_main]
#![no_std]

extern crate alloc;
use standard::{get_args, println};

#[no_mangle]
pub extern "C" fn main() -> isize {
	let args = get_args();
	let message = args.get(0).unwrap_or(&"");
	println!("{}", message);

	return 0;
}
