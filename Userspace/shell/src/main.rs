#![no_main]
#![no_std]

extern crate alloc;
use alloc::{string::ToString, vec::Vec};
use standard::{
	get_args, print, println,
	syscalls::{exec, file_exists, info, kill, quit, read_line, wait},
};

#[no_mangle]
pub extern "C" fn main() -> isize {
	let args = get_args();
	println!("TTY: {}", args[0]);

	loop {
		print!("GuyOS > ");
		let input = read_line();
		match input.as_str() {
			"exit" => {
				break;
			}
			"quit" => {
				quit();
			}
			"ps" => {
				info(0, None);
			}
			s if s.starts_with("kill ") => match s.split_whitespace().nth(1) {
				None => println!("Requires extra arguement: pid"),
				Some(s) => match s.parse() {
					Ok(pid) => kill(pid),
					Err(_) => println!("Pid must be a number!"),
				},
			},
			s if s.starts_with("pcb ") => match s.split_whitespace().nth(1) {
				None => println!("Requires extra arguement: pid"),
				Some(s) => match s.parse() {
					Ok(pid) => info(1, Some(pid)),
					Err(_) => println!("Pid must be a number!"),
				},
			},
			"" => {}
			command => match shell_words::split(command) {
				Ok(v) => {
					let mut tokens = v.as_slice();
					let exec_path = &v[0];
					let mut path = "/bin/".to_string();

					tokens = &tokens[1..];

					let should_wait = match tokens.last() {
						Some(s) if s == "&" => {
							tokens = &tokens[..tokens.len() - 1];
							false
						}
						_ => true,
					};

					let args: Vec<&str> = tokens.iter().map(|s| s.as_str()).collect();
					path.push_str(&exec_path);
					if file_exists(&path) {
						let pid = exec(&path, &args);
						if should_wait {
							match pid {
								Ok(pid) => {
									// println!("{}", pid);
									wait(pid)
								}
								_ => {}
							}
						}
					} else {
						println!("{} does not exist", path);
					}
				}
				Err(e) => {
					println!("Failed to parse command: {}", e)
				}
			},
		}
	}
	return 0;
}
