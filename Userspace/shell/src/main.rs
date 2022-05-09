#![no_main]
#![no_std]

extern crate alloc;
use standard::{
	init, print, println,
	syscalls::{self, read_line},
};

#[no_mangle]
pub extern "C" fn _start() {
	init();
	loop {
		print!("GuyOS > ");
		let input = read_line();
		if input == "quit" {
			break;
		} else {
			println!("ECHO: {}", input);
		}
	}
	syscalls::exit(0);
}
