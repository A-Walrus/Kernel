use alloc::boxed::Box;
use core::alloc::{Allocator, GlobalAlloc, Layout};

use super::paging;
use linked_list_allocator::{Heap, LockedHeap};
use x86_64::{
	addr::VirtAddr,
	structures::paging::{
		page::{Page, PageRangeInclusive},
		PageTableFlags, Size4KiB,
	},
};

/// This is the global allocator. It is automatically used by things like box and vec.
#[global_allocator]
pub static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Starting virtual address of the kernel heap.
const HEAP_START: usize = 0xFFFFD00000000000;

/// Starting virtual address of the kernel uncachable heap.
pub static UNCACHED_ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Starting virtual address of the kernel uncachable heap.
const UNCACHED_HEAP_START: usize = 0xFFFFE00000000000;

/// Error handler automatically called by rust on allocation failiures.
#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
	panic!("allocation error: {:?}", layout)
}

/// Initialize linked list allocator. Enables the usage of the heap by the kernel.
pub fn setup(frambuffer_size: usize) {
	{
		let heap_size = (frambuffer_size + frambuffer_size / 2) + 0x400000; // (The size of the framebuffer * 1.5) + 4 MiB
		let range = PageRangeInclusive::<Size4KiB> {
			start: Page::containing_address(VirtAddr::new(HEAP_START as u64)),
			end: Page::containing_address(VirtAddr::new((HEAP_START + heap_size - 1) as u64)),
		};
		let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
		paging::map_in_current(range, flags);
		unsafe {
			ALLOCATOR.lock().init(HEAP_START, heap_size);
		}
	}

	{
		let uncached_heap_size = 0x400000;
		let range = PageRangeInclusive::<Size4KiB> {
			start: Page::containing_address(VirtAddr::new(UNCACHED_HEAP_START as u64)),
			end: Page::containing_address(VirtAddr::new((UNCACHED_HEAP_START + uncached_heap_size - 1) as u64)),
		};
		let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;
		paging::map_in_current(range, flags);
		unsafe {
			UNCACHED_ALLOCATOR.lock().init(UNCACHED_HEAP_START, uncached_heap_size);
		}
	}
}

/// Like [Box::new_in] for the uncached heap (but it doesn't cause the compiler to crash)
pub fn uncache_box_new<T>(value: T) -> Box<T, &'static LockedHeap> {
	unsafe {
		let raw_ptr = UNCACHED_ALLOCATOR.allocate(Layout::new::<T>()).unwrap().as_mut_ptr();
		let ptr = raw_ptr as *mut T;
		let reference = &mut *ptr;
		*reference = value;
		Box::from_raw_in(ptr, &UNCACHED_ALLOCATOR)
	}
}
