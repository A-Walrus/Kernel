#![no_main]
#![no_std]

extern crate alloc;
use alloc::{string::ToString, vec::Vec};
use standard::{
	print, println,
	syscalls::{exec, file_exists, read_line, wait},
};

#[no_mangle]
pub extern "C" fn main() -> isize {
	loop {
		print!("GuyOS > ");
		let input = read_line();
		let mut split = input.split_ascii_whitespace();
		match split.next() {
			Some("exit") => {
				break;
			}
			Some(exec_path) => {
				let args: Vec<&str> = split.collect();
				let mut path = "/bin/".to_string();
				path.push_str(exec_path);
				if file_exists(&path) {
					let pid = exec(&path, &args);
					match pid {
						Ok(pid) => wait(pid),
						_ => {}
					}
				} else {
					println!("{} does not exist", path);
				}
			}
			None => {}
		}
	}
	return 0;
}
