use core::{
	alloc::{GlobalAlloc, Layout},
	ops::{Deref, DerefMut},
	ptr::slice_from_raw_parts_mut,
};

use super::paging;
use linked_list_allocator::LockedHeap;
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

/// Buffer stored on the uncached heap
pub struct UBuffer {
	/// Pointer to the buffer on the uncached heap
	pub slice: *mut [u8],
}

impl UBuffer {
	/// Create a new uncached buffer of a given size on the heap
	pub fn new(size: usize) -> Self {
		serial_println!("UBuffer size: {}", size);
		let ptr;
		unsafe { ptr = UNCACHED_ALLOCATOR.alloc(Layout::from_size_align(size, 1).unwrap()) };
		Self {
			slice: slice_from_raw_parts_mut(ptr, size),
		}
	}
}

impl Drop for UBuffer {
	fn drop(&mut self) {
		unsafe {
			UNCACHED_ALLOCATOR.dealloc(
				self.slice.as_mut_ptr(),
				Layout::from_size_align(self.slice.len(), 1).unwrap(),
			)
		}
	}
}

/// Box that is allocated and deallocated from the uncached allocator
pub struct UBox<T> {
	ptr: *mut T,
}

impl<T> UBox<T> {
	/// Create a new uncached box around this value. This will be allocated in an uncached area, and deallocated when the [UBox] is dropped.
	/// Since this is using the allocator api, the alignment and layout should be correct, according to the type.
	pub fn new(value: T) -> Self {
		unsafe {
			let ptr = uncached_allocate_value(value);
			serial_println!("Virt UBOX: {:?}", ptr);
			UBox { ptr: ptr }
		}
	}
}

/// Allocate space for a type in an uncached area and initialize it with zeroes.
/// # Safety
/// - This is unsafe because all zeroes may not be a valid value for the type
pub unsafe fn uncached_allocate_zeroed<T>() -> *mut T {
	let raw_ptr = UNCACHED_ALLOCATOR.alloc(Layout::new::<T>());
	let ptr = raw_ptr as *mut T;
	ptr.write_bytes(0, 1);
	ptr
}

/// Allocate room for the value on the uncached heap. Copies the value to the new allocated area.
pub unsafe fn uncached_allocate_value<T>(value: T) -> *mut T {
	let raw_ptr = UNCACHED_ALLOCATOR.alloc(Layout::new::<T>());
	let ptr = raw_ptr as *mut T;
	let reference = &mut *ptr;
	*reference = value;
	ptr
}

impl<T> Deref for UBox<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		unsafe { &*self.ptr }
	}
}

impl<T> DerefMut for UBox<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut *self.ptr }
	}
}

impl<T> Drop for UBox<T> {
	fn drop(&mut self) {
		unsafe { UNCACHED_ALLOCATOR.dealloc(self.ptr as *mut u8, Layout::new::<T>()) }
	}
}
