use crate::io::{IOError, Read, Write};
#[allow(unused_imports)]
use crate::{syscall0, syscall1, syscall2, syscall3, syscall4, syscall5};
use alloc::string::String;
use bitflags::bitflags;

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
pub fn kill(pid: Pid) {
	unsafe {
		syscall1(14, pid as usize);
	}
}

type Pid = usize;

pub fn exec(path: &str, args: &[&str]) -> Result<Pid, ()> {
	let pid = unsafe {
		syscall4(
			3,
			path.as_ptr() as usize,
			path.len(),
			args.as_ptr() as usize,
			args.len(),
		)
	};
	if pid >= 0 {
		Ok(pid as Pid)
	} else {
		Err(())
	}
}

pub fn wait(pid: Pid) {
	unsafe {
		syscall1(10, pid as usize);
	}
}

pub fn open_file(path: &str, flags: OpenFlags) -> Result<Handle, ()> {
	let handle = unsafe { syscall3(5, path.as_ptr() as usize, path.len(), flags.bits() as usize) };
	if handle >= 0 {
		Ok(handle as Handle)
	} else {
		Err(())
	}
}

pub fn sys_paint(x: usize, y: usize, r: u8, g: u8, b: u8) {
	unsafe {
		syscall5(17, x, y, r as usize, g as usize, b as usize);
	}
}

pub fn open_dir(path: &str) -> Result<Handle, ()> {
	let handle = unsafe { syscall2(9, path.as_ptr() as usize, path.len()) };
	if handle >= 0 {
		Ok(handle as Handle)
	} else {
		Err(())
	}
}

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

pub fn file_exists(path: &str) -> bool {
	let f = File::open(path);
	f.is_ok()
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

pub fn quit() -> ! {
	unsafe {
		syscall0(11);
	}
	// This is unreachable but makes compiler happy
	loop {}
}

pub fn rmdir(path: &str) -> Result<(), ()> {
	let res = unsafe { syscall2(13, path.as_ptr() as usize, path.len()) };
	if res < 0 {
		Err(())
	} else {
		Ok(())
	}
}

pub fn mkdir(path: &str) -> Result<(), ()> {
	let res = unsafe { syscall2(15, path.as_ptr() as usize, path.len()) };
	if res < 0 {
		Err(())
	} else {
		Ok(())
	}
}

pub fn info(info_type: usize, arg0: Option<usize>) {
	unsafe {
		syscall2(16, info_type, arg0.unwrap_or(0));
	}
}

pub fn unlink(path: &str) -> Result<(), ()> {
	let res = unsafe { syscall2(12, path.as_ptr() as usize, path.len()) };
	if res < 0 {
		Err(())
	} else {
		Ok(())
	}
}

pub struct Dir(Handle);
impl Dir {
	pub fn open(path: &str) -> Result<Self, ()> {
		let handle = open_dir(path)?;
		Ok(Dir(handle))
	}
}

impl Iterator for Dir {
	type Item = String;
	fn next(&mut self) -> Option<Self::Item> {
		let mut buf = [0; 512];
		let res = read(&mut buf, self.0);
		match res {
			count if count > 0 => {
				let slice = &buf[..count as usize];
				String::from_utf8(slice.to_vec()).ok()
			}
			_ => None,
		}
	}
}

pub struct File(Handle);

impl File {
	pub fn create(path: &str) -> Result<Self, ()> {
		let a = open_file(path, OpenFlags::CREATE)?;
		Ok(File(a))
	}

	pub fn open(path: &str) -> Result<Self, ()> {
		let a = open_file(path, OpenFlags::empty())?;
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

bitflags! {
	/// Flags for opening a file
	pub struct OpenFlags: u64 {
		/// Crate the file if it doesn't exist
		const CREATE = 0b0001;
		/// Truncate file
		const TRUNCATE = 0b0010;
	}
}
