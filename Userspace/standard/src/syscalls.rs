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
		use alloc::format;
		use standard::syscalls::print_a;
		print_a(&format!($($arg)*))
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
