use core::{
	mem::MaybeUninit,
	ptr::{read_volatile, write_volatile},
};

/// Volatile wrapper for a value. Internally uses read and write volatile.
#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct V<T: Copy + Clone> {
	value: MaybeUninit<T>,
}

impl<T: Copy + Clone> V<T> {
	/// Create a new Volatile with the value zeroed
	/// # Safety
	/// This is unsafe because all zeroes may be an undefined state for the type.
	pub unsafe fn zeroed() -> Self {
		Self {
			value: MaybeUninit::zeroed(),
		}
	}

	/// Create a new Volatile with the value uninitialized
	/// # Safety
	/// This is unsafe because uninitialized values are unsafe.
	pub unsafe fn uninit() -> Self {
		Self {
			value: MaybeUninit::uninit(),
		}
	}

	/// Create a new Volatile with a given value
	pub fn from(value: T) -> Self {
		Self {
			value: MaybeUninit::new(value),
		}
	}

	/// Read this value, volatilely
	pub unsafe fn read(&self) -> T {
		read_volatile(self.value.as_ptr())
	}

	/// Write this value, volatilely
	pub unsafe fn write(&mut self, value: T) {
		write_volatile(self.value.as_mut_ptr(), value);
	}
}
