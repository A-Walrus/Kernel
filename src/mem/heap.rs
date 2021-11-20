extern crate alloc;

/// The static global allocator struct.
#[global_allocator]
static ALLOCATOR: Locked<BuddyAllocator> = Locked::new(BuddyAllocator::new());

use crate::{mem::paging, serial_print, serial_println};
use alloc::alloc::{GlobalAlloc, Layout};
use bootloader::boot_info::MemoryRegions;
use core::{cmp::max, mem::size_of, ptr::null_mut};

/// The log of the size of the heap (the size of the heap must be a power of 2)
const LOG_HEAP_SIZE: usize = 23; // 8MB
/// The actual size of the heap (2 <sup> [LOG_HEAP_SIZE] </sup>)
const HEAP_SIZE: usize = 1 << LOG_HEAP_SIZE;
/// The starting (virtual) address of the heap.
const HEAP_START: usize = 0xFFFF980000000000;

/// The log of the size of the smallest block (the sizes of all the blocks are powers of two.
const LOG_SMALLEST_SIZE: usize = 5;
/// The size of the smallest block (2 <sup> [LOG_SMALLEST_SIZE] </sup>).
/// This value must be big enough so that a [Node] will fit in it.
const SMALLEST_SIZE: usize = 1 << LOG_SMALLEST_SIZE;
/// The number of layers (block sizes) in this buddy allocator heap.
const LAYERS: usize = LOG_HEAP_SIZE - LOG_SMALLEST_SIZE;
/// Array of the size of the blocks in each layer.
const SIZES: [usize; LAYERS] = {
	let mut size = HEAP_SIZE;
	let mut layers = [0; LAYERS];
	let mut i = 0;
	while i < LAYERS {
		layers[i] = size;
		size = size >> 1;
		i += 1;
	}
	layers
};

// Block IDs
// +-------------------------------+
// |               0               | Layer 0
// +---------------+---------------+
// |       1       |       2       | Layer 1
// +-------+-------+-------+-------+
// |   3   |   4   |   5   |   6   | Layer 2
// +---+---+---+---+---+---+---+---+
// | 7 | 8 | 9 | A | B | C | D | E | Layer 3
// +---+---+---+---+---+---+---+---+

/// Set up heap mapping, and heap allocator.
pub fn setup(memory_regions: &'static MemoryRegions) {
	// Make sure that the smallest size is big enough to hold a node.
	assert!(
		size_of::<Node>() <= SMALLEST_SIZE,
		"Smallest size is too small enough to fit a Node"
	);

	paging::map_heap(memory_regions, HEAP_START, HEAP_SIZE);
	ALLOCATOR.lock().init();
}

/// The connection between nodes in a doubly linked list
type Link = Option<usize>;

/// A node in a doubly linked list.
#[derive(Debug, Copy, Clone)]
struct Node {
	/// A link to the previous node.
	prev: Link,
	/// A link to the next node.
	next: Link,
}

/// The number of pairs of buddys (2 <sup> [LAYERS] </sup> - 1).
const BUDDY_PAIRS: usize = (1 << LAYERS) - 1;

/// Struct representing the global allocator. It implements [GlobalAlloc] and is
/// set as the [global_allocator].
pub struct BuddyAllocator {
	/// For each pair of buddys (a,b) a_free XOR b_free. [BUDDY_PAIRS] stores the number of pairs
	/// of buddys.
	// TODO store each value as a single bit, and not a full byte.
	xor_free: [bool; BUDDY_PAIRS],

	/// linked list of free blocks for every layer.
	linked_lists: [Node; LAYERS],
}

impl BuddyAllocator {
	/// Constructor for a [BuddyAllocator]. **Does not add any free blocks to any list**.
	const fn new() -> Self {
		let empty = Node { next: None, prev: None };
		BuddyAllocator {
			xor_free: [false; BUDDY_PAIRS],
			linked_lists: [empty; LAYERS],
		}
	}

	/// Add the entire heap as a free block of layer 0.
	fn init(&mut self) {
		self.add_free_block(0, false)
	}

	/// returns index into [BuddyAllocator::linked_lists], which holds the smallest blocks big
	/// enough to store something of the ```wanted_size```.
	fn layer_from_size(wanted_size: usize) -> usize {
		if wanted_size <= SMALLEST_SIZE {
			return LAYERS - 1;
		}
		let log = ((wanted_size - 1).log2() + 1) as usize;
		LAYERS + LOG_SMALLEST_SIZE - log - 1
	}

	/// Get the index of a block of a certain address in a certain layer.
	fn get_index_in_layer(layer: usize, addr: usize) -> usize {
		(addr - HEAP_START) / SIZES[layer]
	}

	/// Get the id of a block by its layer and address.
	fn get_id(layer: usize, addr: usize) -> usize {
		let start_of_layer = (1 << layer) - 1;
		BuddyAllocator::get_index_in_layer(layer, addr) + start_of_layer
	}

	/// Get the id of a blocks buddy.
	fn get_buddy_id(id: usize) -> usize {
		id + ((id % 2) * 2) - 1
	}

	/// Get a pointer to a block from its id. Can be returned as result of allocation request, or
	/// cast into a [Node] to edit linked list.
	fn id_to_ptr(id: usize) -> *mut u8 {
		let addr = HEAP_START;
		let layer = BuddyAllocator::layer_from_id(id);
		let pos_in_layer = BuddyAllocator::pos_in_layer(id, layer);
		(addr + (SIZES[layer] * pos_in_layer)) as *mut u8
	}

	/// Get a reference to the node at a given id.
	unsafe fn node_at_id(id: usize) -> &'static mut Node {
		&mut *(BuddyAllocator::id_to_ptr(id) as *mut Node)
	}

	/// Returns the index of the buddy pair in [BuddyAllocator::xor_free]
	fn pair_id(id: usize) -> usize {
		(id - 1) / 2
	}

	/// Get the layer a certain id belongs to.
	fn layer_from_id(id: usize) -> usize {
		((id + 1).log2()) as usize
	}

	/// Get the position of a block with a certain id, inside its layer.
	fn pos_in_layer(id: usize, layer: usize) -> usize {
		let start_of_layer = (1 << layer) - 1;
		id - start_of_layer
	}

	/// Get the ids of the children of a certain block.
	fn get_children_ids(id: usize) -> [usize; 2] {
		[(id * 2) + 1, (id * 2) + 2]
	}

	/// Get the id of a blocks parent.
	/// # Panics
	/// If called with id = 0, which doesn't have a parent, it will panic.
	fn get_parent_id(id: usize) -> usize {
		(id - 1) / 2
	}

	/// Get a block of a given layer, returns block id
	fn get_block(&mut self, layer: usize) -> usize {
		serial_println!("GET AT LAYER {}", layer);
		// Check if there is a block at the wanted layer:
		match self.linked_lists[layer].next {
			Some(next) => {
				let id = BuddyAllocator::get_id(layer, next as usize);
				unsafe {
					self.remove_block(id);
				}
				id
			}
			None => {
				if layer == 0 {
					panic!("No more heap");
				}
				// Get a block the next size up
				let parent_id = self.get_block(layer - 1);
				let children_ids = BuddyAllocator::get_children_ids(parent_id);

				self.add_free_block(children_ids[0], false);
				self.add_free_block(children_ids[1], false);

				unsafe {
					self.remove_block(children_ids[0]);
				}
				children_ids[0]
			}
		}
	}

	/// Take a block of a given id. This will remove it from the linked list, by relinking the
	/// previous and next nodes. It will also update the [BuddyAllocator::xor_free] array.
	unsafe fn remove_block(&mut self, id: usize) {
		serial_println!("REMOVE {}", id);

		if id != 0 {
			let pair_id = BuddyAllocator::pair_id(id);
			self.xor_free[pair_id] = !self.xor_free[pair_id];
		}

		let node = BuddyAllocator::node_at_id(id);
		match node.prev {
			Some(prev) => {
				(*(prev as *mut Node)).next = node.next;
				match node.next {
					Some(next) => {
						(*(next as *mut Node)).prev = Some(prev);
					}
					None => {}
				}
			}
			None => {
				// This should never happen.
				unreachable!("Tried to remove first node, this shouldn't happen");
			}
		}
	}

	/// Add a free block of a certain id into the proper linked list. It will push it to the front
	/// of the list, and update the [BuddyAllocator::xor_free] array. If ```with_merge``` is true,
	/// it will attempt to merge it with its buddy, and if it can, recursively call itself on their
	/// combined parent block.
	fn add_free_block(&mut self, id: usize, with_merge: bool) {
		serial_println!("ADD FREE {}", id);
		// Check if it's buddy is free
		if with_merge && id != 0 && self.xor_free[BuddyAllocator::pair_id(id)] {
			// Buddy is free, can merge
			unsafe {
				self.remove_block(BuddyAllocator::get_buddy_id(id));
			}

			let parent_id = BuddyAllocator::get_parent_id(id);
			self.add_free_block(parent_id, true);
		} else {
			// Buddy isn't free
			if id != 0 {
				let pair_id = BuddyAllocator::pair_id(id);
				self.xor_free[pair_id] = !self.xor_free[pair_id];
			}

			let layer = BuddyAllocator::layer_from_id(id);
			let this_node: &mut Node;
			unsafe {
				this_node = BuddyAllocator::node_at_id(id);
			}
			this_node.next = self.linked_lists[layer].next;
			this_node.prev = Some(&mut self.linked_lists[layer] as *mut _ as usize);
			self.linked_lists[layer].next = Some(this_node as *mut _ as usize);
			match this_node.next {
				Some(next) => unsafe {
					(*(next as *mut Node)).prev = Some(this_node as *mut _ as usize);
				},
				None => {}
			}
		}
	}
}

unsafe impl GlobalAlloc for Locked<BuddyAllocator> {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		let mut allocator = self.lock();
		let layer = BuddyAllocator::layer_from_size(layout.size());
		BuddyAllocator::id_to_ptr(allocator.get_block(layer))
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		let mut allocator = self.lock();
		let layer = BuddyAllocator::layer_from_size(layout.size());
		allocator.add_free_block(BuddyAllocator::get_id(layer, ptr as usize), true)
	}
}

/// The error handler for allocation errors. Called automatically by rust. It just prints the
/// requested layout that caused the allocation fail.
#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
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
