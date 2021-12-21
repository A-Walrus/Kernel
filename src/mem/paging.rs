use super::buddy;
use crate::{serial_print, serial_println};
use bootloader::boot_info::{MemoryRegionKind, MemoryRegions};
use core::iter;
use x86_64::{
	addr::{PhysAddr, VirtAddr},
	registers::control::Cr3,
	structures::paging::{
		mapper::{CleanUp, Mapper, OffsetPageTable},
		page::{Page, PageRangeInclusive},
		page_table::{PageTableEntry, PageTableFlags},
		FrameAllocator, FrameDeallocator, PageTable, PhysFrame, Size1GiB, Size2MiB, Size4KiB,
	},
};

/// Virtual address that the entire physical memory is mapped starting from.
const PHYSICAL_MAPPING_OFFSET: u64 = 0xFFFFC00000000000;

/// Map the given range of pages, to anywhere in the physical memory, on the current page table. Allocate
/// frames for them.
pub fn map_in_current(range: PageRangeInclusive) {
	let table = get_current_page_table();
	map(range, table);
}

/// Map the given range of pages, to anywhere in the physical memory, on the given page table. Allocate
/// frames for them.
pub fn map(range: PageRangeInclusive, table: &mut PageTable) {
	let mut offset_table: OffsetPageTable;
	unsafe {
		offset_table = get_offset_page_table(table);
	}
	let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
	let mut frame_allocator = buddy::ALLOCATOR.lock();
	for page in range {
		unsafe {
			let frame = frame_allocator.allocate_frame().unwrap();
			let result = offset_table.map_to(page, frame, flags, &mut *frame_allocator);
			match result {
				Ok(flush) => flush.flush(),
				Err(e) => serial_println!("Failed to map! {:?}", e),
			}
		}
	}
}

/// set up paging. Clean up the page table created by the bootloader.
pub fn setup() {
	let table = get_current_page_table();

	// This is kind of a memory leak, sort of...
	for entry in table.iter_mut().take(256).filter(|entry| !entry.is_unused()) {
		entry.set_unused()
	}
}

unsafe fn wipe_lower_half(table: &mut PageTable) {
	for entry in table.iter_mut().take(256).filter(|entry| !entry.is_unused()) {
		wipe_recursive(entry, 4);
	}
}

unsafe fn wipe_recursive(entry: &mut PageTableEntry, depth: usize) {
	let sub_table = get_sub_table_mut(entry);
	if depth > 1 {
		match sub_table {
			Err(SubPageError::HugePage) => unsafe {
				match depth {
					3 => {
						buddy::ALLOCATOR
							.lock()
							.deallocate_frame(PhysFrame::<Size1GiB>::from_start_address(entry.addr()).unwrap());
					}
					2 => {
						buddy::ALLOCATOR
							.lock()
							.deallocate_frame(PhysFrame::<Size2MiB>::from_start_address(entry.addr()).unwrap());
					}
					_ => {
						unreachable!("The only depths with huge pages are 2 and 3");
					}
				}
			},
			Ok(table) => {
				for sub_entry in table.iter_mut().filter(|entry| !entry.is_unused()) {
					wipe_recursive(sub_entry, depth - 1);
				}
				unsafe {
					buddy::ALLOCATOR
						.lock()
						.deallocate_frame(PhysFrame::<Size4KiB>::from_start_address(entry.addr()).unwrap());
				}
			}
			Err(SubPageError::EntryUnused) => {
				unreachable!("Tried to wipe entry that is unused");
			}
		}
	} else {
		unsafe {
			buddy::ALLOCATOR
				.lock()
				.deallocate_frame(PhysFrame::<Size4KiB>::from_start_address(entry.addr()).unwrap());
		}
	}
	entry.set_unused();
}

/// Translate physical address to virtual address by adding constant [PHYSICAL_MAPPING_OFFSET].
pub fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
	VirtAddr::new(phys.as_u64() + PHYSICAL_MAPPING_OFFSET)
}

/// Translate virtual address **in the offset mapped area* to physical address by subtracting
/// constant [PHYSICAL_MAPPING_OFFSET].
pub fn virt_to_phys(virt: VirtAddr) -> PhysAddr {
	PhysAddr::new(virt.as_u64() - PHYSICAL_MAPPING_OFFSET)
}

/// Returns a reference (with static lifetime) to the current top level page table.
pub fn get_current_page_table() -> &'static mut PageTable {
	let (phys_frame, _flags) = Cr3::read(); // CR3 register stores location of page table (and some flags)
	let phys_addr = phys_frame.start_address();

	// This is sound because we know that CR3 points to a page table
	unsafe { get_page_table_by_addr(phys_addr) }
}

/// Get an [OffsetPageTable] from a page table. This is a wrapper which makes it easy to work with
/// page tables that have mapped the entire physical memory to some offset (in this case
/// [PHYSICAL_MAPPING_OFFSET]).
/// ## Safety
/// The caller must insure:
/// * The page table is a level 4 table
/// * The entire physical memory is mapped in this page table at [PHYSICAL_MAPPING_OFFSET].
pub unsafe fn get_offset_page_table(page_table: &mut PageTable) -> OffsetPageTable {
	let offset = VirtAddr::new(PHYSICAL_MAPPING_OFFSET);
	unsafe { OffsetPageTable::new(page_table, offset) }
}

/// Get a reference to the page table at a certain physical address.
/// ## Safety
/// This function is unsafe because it will read the data at whatever physical address you give it.
/// Make sure that this is the physical address of a page table.
unsafe fn get_page_table_by_addr(addr: PhysAddr) -> &'static mut PageTable {
	let virt_addr = phys_to_virt(addr);
	let table_ptr = virt_addr.as_mut_ptr();
	let page_table: &'static mut PageTable;
	unsafe {
		page_table = &mut *table_ptr;
	}
	page_table
}

/// Print all not-empty entries in a page table
pub fn print_table(page_table: &PageTable) {
	for (i, entry) in page_table.iter().enumerate() {
		if !entry.is_unused() {
			serial_println!("L4 Entry {}: {:?}", i, entry);
		}
	}
}

/// Recursively print page table.
pub fn print_table_recursive(page_table: &PageTable, depth: usize) {
	const PADDINGS: [&str; 4] = ["", "\t", "\t\t", "\t\t\t"];
	for (i, entry) in page_table.iter().enumerate() {
		if !entry.is_unused() {
			let padding = PADDINGS[4 - depth];
			serial_println!("{}L{} Entry {}: {:?}", padding, depth, i, entry);
			if depth > 1 {
				let sub_table = get_sub_table(&page_table[i]);
				if let Ok(table) = sub_table {
					print_table_recursive(table, depth - 1);
				}
			}
		}
	}
}

/// Error returned by [get_sub_table]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SubPageError {
	/// The entry is unused.
	EntryUnused,
	/// The entry is not a page table, its a huge page.
	HugePage,
}

/// Gets the sub table at a certain index in a page table, where `0 ≤ index < 512`. If the entry is
/// unused, or is a huge page and not a page table, an error will be returned.
pub fn get_sub_table<'a>(entry: &PageTableEntry) -> Result<&'a PageTable, SubPageError> {
	if entry.is_unused() {
		Err(SubPageError::EntryUnused)
	} else if entry.flags().contains(PageTableFlags::HUGE_PAGE) {
		Err(SubPageError::HugePage)
	} else {
		let phys_addr = entry.addr();
		Ok(unsafe { get_page_table_by_addr(phys_addr) })
	}
}

/// Gets the sub table at a certain index in a page table, where `0 ≤ index < 512`. If the entry is
/// unused, or is a huge page and not a page table, an error will be returned.
pub fn get_sub_table_mut<'a>(entry: &mut PageTableEntry) -> Result<&'a mut PageTable, SubPageError> {
	if entry.is_unused() {
		Err(SubPageError::EntryUnused)
	} else if entry.flags().contains(PageTableFlags::HUGE_PAGE) {
		Err(SubPageError::HugePage)
	} else {
		let phys_addr = entry.addr();
		Ok(unsafe { get_page_table_by_addr(phys_addr) })
	}
}

/// Provides frames for mapper to map. Should be used for the kernel, during boot process. These
/// are gotten from the bootloader's [MemoryRegions] map.
pub struct BootFrameAllocator {
	memory_regions: &'static MemoryRegions,
	next: usize,
}

impl BootFrameAllocator {
	/// Create a new boot frame allocator.
	/// ## Safety
	/// The caller must guarantee:
	/// * The memory map is valid (otherwise the allocator might allocate frames that are in use /
	/// don't exist).
	/// * This is only called once (only one boot frame allocator is constructed) otherwise the
	/// allocators would be allocating the same regions multiple times.
	pub unsafe fn new(memory_regions: &'static MemoryRegions) -> Self {
		BootFrameAllocator {
			memory_regions,
			next: 0,
		}
	}

	fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
		// get usable regions from memory map
		let regions = self.memory_regions.iter();
		let usable_regions = regions.filter(|r| r.kind == MemoryRegionKind::Usable);
		// map each region to its address range
		let addr_ranges = usable_regions.map(|r| r.start..r.end);
		// transform to an iterator of frame start addresses
		let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
		// create `PhysFrame` types from the start addresses
		frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
	}
}

unsafe impl FrameAllocator<Size4KiB> for BootFrameAllocator {
	fn allocate_frame(&mut self) -> Option<PhysFrame> {
		let frame = self.usable_frames().nth(self.next);
		self.next += 1;
		frame
	}
}
