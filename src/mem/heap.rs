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
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

const HEAP_START: usize = 0xFFFFD00000000000;
const HEAP_SIZE: usize = 0x200000; // ;2MiB

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
	panic!("allocation error: {:?}", layout)
}

/// Initialize linked list allocator. Enables the usage of the heap by the kernel.
pub fn setup() {
	let range = PageRangeInclusive::<Size4KiB> {
		start: Page::containing_address(VirtAddr::new(HEAP_START as u64)),
		end: Page::containing_address(VirtAddr::new((HEAP_START + HEAP_SIZE - 1) as u64)),
	};
	serial_println!("About to map");
	paging::map_in_current(range);
	serial_println!("Just mapped, about to initialize");
	unsafe {
		ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
	}
}
