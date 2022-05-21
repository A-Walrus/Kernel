#![no_main]
#![no_std]

extern crate alloc;
use alloc::{string::ToString, vec::Vec};
use standard::{
	print, println,
	syscalls::{exec, file_exists, read_line, wait},
};

use logos::{Lexer, Logos};

#[derive(Logos, Debug, PartialEq)]
enum Token {
	#[token("&")]
	Asynchronous,

	#[regex("[a-zA-Z]+")]
	#[regex("\"[a-zA-Z]+\"")]
	#[regex("'[a-zA-Z]+'")]
	Text,

	#[error]
	Error,

	#[regex(r"[ \t\n\f]+", logos::skip)]
	WhiteSpace,
}

#[no_mangle]
pub extern "C" fn main() -> isize {
	loop {
		print!("GuyOS > ");
		let input = read_line();
		match input.as_str() {
			"" => {}
			"exit" => {
				break;
			}
			line => {
				let lex = Token::lexer(line);
				for item in lex {
					println!("{:?}", item);
				}

				// let args: Vec<&str> = split.collect();
				// let mut path = "/bin/".to_string();
				// path.push_str(line);
				// if file_exists(&path) {
				// 	let pid = exec(&path, &args);
				// 	match pid {
				// 		Ok(pid) => wait(pid),
				// 		_ => {}
				// 	}
				// } else {
				// 	println!("{} does not exist", path);
				// }
			}
		}
	}
	return 0;
}
