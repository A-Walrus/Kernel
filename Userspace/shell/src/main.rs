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
		let mut split = input.split_ascii_whitespace();
		match split.next() {
			Some("quit") => {
				break;
			}
			Some("open") => match split.next() {
				Some(path) => syscalls::open_file(path),
				None => {
					println!("More args needed")
				}
			},
			Some(s) => {
				println!("Invalid command!");
			}
			None => {}
		}
	}
	syscalls::exit(0);
}
