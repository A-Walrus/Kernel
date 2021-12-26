extern crate alloc;
use super::paging;
use crate::{serial_print, serial_println};
use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use linked_list_allocator::LockedHeap;
use x86_64::{
	addr::VirtAddr,
	structures::paging::{
		page::{Page, PageRangeInclusive},
		Size4KiB,
	},
};

/// This is the global allocator. It is automatically used by things like box and vec.
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Starting virtual address of the kernel heap.
const HEAP_START: usize = 0xFFFFD00000000000;

/// Error handler automatically called by rust on allocation failiures.
#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
	panic!("allocation error: {:?}", layout)
}

/// Initialize linked list allocator. Enables the usage of the heap by the kernel.
pub fn setup(frambuffer_size: usize) {
	let heap_size = (frambuffer_size + frambuffer_size / 2) + 400000; // (The size of the framebuffer * 1.5) + 4 MiB
	let range = PageRangeInclusive::<Size4KiB> {
		start: Page::containing_address(VirtAddr::new(HEAP_START as u64)),
		end: Page::containing_address(VirtAddr::new((HEAP_START + heap_size - 1) as u64)),
	};
	paging::map_in_current(range);
	unsafe {
		ALLOCATOR.lock().init(HEAP_START, heap_size);
	}
}
