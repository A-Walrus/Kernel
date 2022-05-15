#![no_main]
#![no_std]

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
			Some("exec") => match split.next() {
				Some(path) => {
					let args: Vec<&str> = split.collect();
					exec(path, &args)
				}
				None => {
					println!("More args needed")
				}
			},
			Some(s) => {
				println!("Invalid command! {}", s);
			}
			None => {}
		}
	}
	return 0;
}
