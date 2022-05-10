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
			Some("print") => match split.next() {
				Some(path) => {
					let handle = syscalls::open_file(path);
					if let Ok(handle) = handle {
						let mut buffer = [0; 128];
						syscalls::read(&mut buffer, handle);
						println!("{:?}", buffer);
					}
				}
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
