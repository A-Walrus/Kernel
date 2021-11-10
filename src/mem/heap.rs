extern crate alloc;

#[global_allocator]
static ALLOCATOR: Locked<BuddyAllocator> = Locked::new(BuddyAllocator::new());

use crate::{mem::paging, serial_println};
use alloc::alloc::{GlobalAlloc, Layout};
use bootloader::boot_info::MemoryRegions;
use core::ptr::null_mut;

const LOG_HEAP_SIZE: usize = 24; // 16MB
const HEAP_SIZE: usize = 1 << LOG_HEAP_SIZE;
const HEAP_START: usize = 0xFFFF980000000000;

const LOG_SMALLEST_SIZE: usize = 3;
const SMALLEST_SIZE: usize = 1 << LOG_SMALLEST_SIZE;
const SIZES: usize = LOG_HEAP_SIZE - LOG_SMALLEST_SIZE + 1;

/// Set up heap mapping, and heap allocator.
pub fn setup(memory_regions: &'static MemoryRegions) {
	paging::map_heap(memory_regions, HEAP_START, HEAP_SIZE);
	ALLOCATOR.lock().init();
}

/// Struct representing the global allocator. It implements [GlobalAlloc] and is
/// set as the [global_allocator].
pub struct BuddyAllocator {
	/// linked list for every size
	linked_lists: [Node; SIZES],
}

type Node = Option<usize>;

impl BuddyAllocator {
	const fn new() -> Self {
		let mut linked_lists = [None; SIZES];
		linked_lists[0] = Some(HEAP_START);
		Self { linked_lists }
	}

	fn init(&mut self) {
		unsafe {
			let root = &mut *(HEAP_START as *mut Node);
			*root = None;
		}
	}

	// returns index into the [BuddyAllocator::linked_lists], which corrosponds to the proper size
	// for this type.
	fn find_size_index(wanted_size: usize) -> usize {
		let mut size = SMALLEST_SIZE;
		let mut i = SIZES - 1;
		while wanted_size > size {
			size = size << 1;
			i -= 1;
		}
		i
	}

	fn buddy(ptr: usize, size_index: usize) -> usize {
		ptr ^ (1 << SIZES - size_index)
	}

	fn get_region(&mut self, size_index: usize) -> *mut u8 {
		if self.linked_lists[size_index].is_some() {
			let node = self.linked_lists[size_index].take();

			if let Some(next_addr) = node {
				let next: &mut Node;
				unsafe { next = &mut *(next_addr as *mut Node) }
				self.linked_lists[size_index] = next.take();
				return next as *mut Node as *mut u8;
			} else {
				panic!("This should never happen!");
			}
		} else {
			let parent_region = self.get_region(size_index - 1);
			// Get both halfs of parent region
			let first_half = parent_region;
			let last_half = BuddyAllocator::buddy(first_half as usize, size_index);
			// Add one half onto the list
			self.add_region(last_half, size_index);
			// Return the other half
			return first_half;
		}
	}

	fn add_region(&mut self, new_addr: usize, size_index: usize) {
		let mut current_node: &mut Node = &mut self.linked_lists[size_index];
		while let Some(addr) = current_node {
			if new_addr < *addr {
				unsafe {
					let next: &mut Node = &mut *(new_addr as *mut Node);
					*next = Some(*addr);
				}
				break;
			} else {
				unsafe {
					current_node = &mut *(*addr as *mut Node);
				}
			}
		}
		*current_node = Some(new_addr);
	}
}

unsafe impl GlobalAlloc for Locked<BuddyAllocator> {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		let size_index = BuddyAllocator::find_size_index(layout.size());
		let mut allocator = self.lock();
		//serial_println!("{:?}", allocator.linked_lists);
		allocator.get_region(size_index)
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		let size_index = BuddyAllocator::find_size_index(layout.size());
		let mut allocator = self.lock();
		allocator.add_region(ptr as usize, size_index);
	}
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
	panic!("allocation error: {:?}", layout)
}

/// A wrapper around [spin::Mutex] to permit trait implementations.
pub struct Locked<A> {
	inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
	/// Create a new locked struct with a certain inner
	pub const fn new(inner: A) -> Self {
		Locked {
			inner: spin::Mutex::new(inner),
		}
	}

	/// lock self and gain access to inner through a [spin::MutexGuard].
	pub fn lock(&self) -> spin::MutexGuard<A> {
		self.inner.lock()
	}
}
