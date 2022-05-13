use crate::io::{IOError, Read, Write};
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
	() => ($crate::syscalls::print_a("\n"));
	($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(
        concat!($fmt, "\n"), $($arg)*))

}

pub fn exit(status: isize) -> ! {
	unsafe {
		syscall1(2, status as usize);
	}
	// This is unreachable but makes compiler happy
	loop {}
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

type Handle = u32;

pub fn close(handle: Handle) {
	unsafe {
		syscall1(7, handle as usize);
	}
}

pub fn read(buffer: &mut [u8], handle: Handle) -> i64 {
	unsafe { syscall3(6, buffer.as_ptr() as usize, buffer.len(), handle as usize) }
}

pub fn write(buffer: &[u8], handle: Handle) -> i64 {
	unsafe { syscall3(8, buffer.as_ptr() as usize, buffer.len(), handle as usize) }
}

pub struct File(Handle);

impl File {
	pub fn new(path: &str) -> Result<Self, ()> {
		let a = open_file(path)?;
		Ok(File(a))
	}
}

impl Read for File {
	fn read(&mut self, buf: &mut [u8]) -> Result<usize, IOError> {
		match read(buf, self.0) {
			count if count >= 0 => Ok(count as usize),
			_ => Err(IOError::Other),
		}
	}
}

impl Write for File {
	fn write(&mut self, buf: &[u8]) -> Result<usize, IOError> {
		match write(buf, self.0) {
			count if count >= 0 => Ok(count as usize),
			_ => Err(IOError::Other),
		}
	}
	fn flush(&mut self) -> Result<(), IOError> {
		// No need to flush
		Ok(())
	}
}

impl Drop for File {
	fn drop(&mut self) {
		close(self.0)
	}
}

pub fn open_file(path: &str) -> Result<Handle, ()> {
	let handle = unsafe { syscall2(5, path.as_ptr() as usize, path.len()) };
	if handle >= 0 {
		Ok(handle as Handle)
	} else {
		Err(())
	}
}

use alloc::string::String;

pub fn read_line() -> String {
	let mut s = String::new();
	loop {
		let mut buf = [0];
		get_input(&mut buf);
		let char = buf[0] as char;
		match char {
			'\n' => {
				print!("\n");
				break;
			}
			'\x08' => {
				if s.pop().is_some() {
					print!("\x08");
				}
			}
			char if !char.is_ascii_control() => {
				print!("{}", char);
				s.push(char);
			}
			_ => {}
		}
	}
	s
}
