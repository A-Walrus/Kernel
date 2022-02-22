use alloc::vec::Vec;
use core::{
	mem::{size_of, zeroed},
	ptr::slice_from_raw_parts_mut,
};

/// Error from IO
pub enum IOError {}

/// Trait allowing reading from a stream
pub trait Read {
	/// Read to a buffer. Number of bytes actually read in result.
	fn read(&mut self, buf: &mut [u8]) -> Result<usize, IOError>;

	/// Read till end of stream
	fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize, IOError> {
		let start_len = buf.len();
		let mut buffer: [u8; 512] = [0; 512];
		loop {
			let result = self.read(&mut buffer);
			match result {
				Ok(0) => return Ok(buf.len() - start_len),
				Ok(n) => buf.extend_from_slice(&buffer[..n]),
				Err(e) => {
					return Err(e);
				}
			}
		}
	}

	/// Read data into a struct.
	/// # Safety
	/// - Must make sure that the data in that part of the disk is valid for that type, otherwise
	/// UB
	// TODO make failable
	#[inline(always)]
	unsafe fn read_type<T>(&mut self) -> T {
		let mut val: T = zeroed(); // TODO get rid of unnecessary zeroeization
		let slice = &mut *(slice_from_raw_parts_mut(&mut val as *mut T as *mut u8, size_of::<T>()));
		self.read(slice);
		val
	}
}
