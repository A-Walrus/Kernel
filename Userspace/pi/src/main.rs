#![no_main]
#![no_std]
#![feature(asm)]

extern crate alloc;
use standard::{get_args, println};

#[no_mangle]
pub extern "C" fn main() -> isize {
	let args = get_args();

	let n: usize = match args.get(0).map(|s| s.parse()) {
		None => 500000,
		Some(Ok(num)) => num,
		Some(Err(_)) => {
			println!("Arguement must be a number!");
			return -1;
		}
	};

	let mut sum: f64 = 0.;
	for i in 1..=n {
		let val = 1. / ((2. * i as f64) - 1.);
		if i % 2 == 0 {
			sum -= val;
		} else {
			sum += val;
		}
	}

	let pi = sum * 4.;
	println!("PI: {} (calculated with {} iterations)", pi, n);
	return 0;
}
