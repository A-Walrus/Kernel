#![no_main]
#![no_std]

extern crate alloc;
use standard::{init, println, syscalls};

#[no_mangle]
pub extern "C" fn _start() {
	init();
	loop {
		println!("GuyOS > ");
		let mut buf = [0; 16];
		syscalls::get_input(&mut buf);
		println!("{:?}", buf);
	}
	syscalls::exit(0);
}
