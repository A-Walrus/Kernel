#[allow(unused_imports)]
use crate::{syscall0, syscall1, syscall2, syscall3, syscall4, syscall5};

pub fn print_a(s: &str) {
	unsafe {
		syscall2(1, s.as_ptr() as usize, s.len());
	}
}

#[macro_export]
macro_rules! print {
	($($arg:tt)*) => {
		$crate::syscalls::print_a(&alloc::format!($($arg)*))
	};
}

#[macro_export]
macro_rules! println {
	() => (print_a("\n"));
	($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(
        concat!($fmt, "\n"), $($arg)*))

}

pub fn exit(status: usize) {
	unsafe {
		syscall1(2, status);
	}
}

pub fn exec(path: &str) {
	unsafe {
		syscall2(3, path.as_ptr() as usize, path.len());
	}
}

pub fn get_input(buffer: &mut [u8]) {
	unsafe {
		syscall2(4, buffer.as_ptr() as usize, buffer.len());
	}
}

pub fn open_file(path: &str) {
	unsafe {
		syscall2(5, path.as_ptr() as usize, path.len());
	}
}

use alloc::string::String;

pub fn read_line() -> String {
	let mut s = String::new();
	loop {
		let mut buf = [0];
		get_input(&mut buf);
		let char = buf[0] as char;
		print!("{}", char);
		if char == '\n' {
			break;
		} else {
			s.push(char);
		}
	}
	s
}
