extern crate alloc;
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

/// Zero sized struct representing the global allocator. It implements [GlobalAlloc] which allowes
/// set as the [global_allocator].
pub struct Allocator;

unsafe impl GlobalAlloc for Allocator {
	unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
		// Signal an error by returning a null ptr
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

/// Initialize linked list allocator. Enables the usage of the heap by the kernel.
pub fn setup() {
	unsafe {
		ALLOCATOR.lock().init(0xFFFF980000000000, 4096);
	}
}
