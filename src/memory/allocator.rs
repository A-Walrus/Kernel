extern crate alloc;

#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

pub struct Allocator;

unsafe impl GlobalAlloc for Allocator {
	unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
		null_mut()
	}

	unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
		panic!("dealloc should be never called")
	}
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
	panic!("allocation error: {:?}", layout)
}
