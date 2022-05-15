#![no_main]
#![no_std]
#![feature(str_split_whitespace_as_str)]

extern crate alloc;
use alloc::{string::String, vec::Vec};
use standard::{
	io::Read,
	print, println,
	syscalls::{exec, read_line, File},
};

#[no_mangle]
pub extern "C" fn main() -> isize {
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
					let file = File::new(path);
					match file {
						Ok(mut f) => {
							let mut buf = Vec::new();
							f.read_to_end(&mut buf).expect("Failed to read!");
							let res = String::from_utf8(buf);
							match res {
								Ok(string) => println!("{}", string),
								Err(_) => println!("File is not UTF8"),
							}
						}
						Err(_) => {
							println!("Failed to open file")
						}
					}
				}
				None => {
					println!("More args needed")
				}
			},
			Some("exec") => {
				exec(split.as_str());
				// match split.next() {
				// Some(path) => exec(path),
				// None => {
				// 	println!("More args needed")
				// }
				println!("hi");
			}
			Some(s) => {
				println!("Invalid command! {}", s);
			}
			None => {}
		}
	}
	return 0;
}
