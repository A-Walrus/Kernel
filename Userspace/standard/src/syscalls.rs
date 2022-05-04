#[allow(unused_imports)]
use crate::{syscall0, syscall1, syscall2, syscall3, syscall4, syscall5};

pub fn print(s: &str) {
	unsafe {
		syscall2(1, s.as_ptr() as usize, s.len());
	}
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
