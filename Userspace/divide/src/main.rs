#![no_main]
#![no_std]
#![feature(asm)]

extern crate alloc;
use standard::{get_args, println};

#[no_mangle]
pub extern "C" fn main() -> isize {
	let args = get_args();

	if args.len() != 2 {
		println!("Must input two arguements!");
		return -1;
	}

	let a: usize = match args.get(0).map(|s| s.parse()) {
		Some(Ok(num)) => num,
		_ => {
			println!("Arguement must be a number!");
			return -1;
		}
	};
	let b: usize = match args.get(1).map(|s| s.parse()) {
		Some(Ok(num)) => num,
		_ => {
			println!("Arguement must be a number!");
			return -1;
		}
	};

	let mut res = a;
	unsafe {
		asm!(
			"mov edx, 0",
			"div ecx",
			inout("eax") res,
			in("ecx") b,
		);
	}

	println!("{} / {} = {}", a, b, res);
	return 0;
}
