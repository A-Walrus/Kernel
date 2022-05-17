use super::buddy;
use crate::serial_println;
use alloc::boxed::Box;
use bootloader::boot_info::{MemoryRegionKind, MemoryRegions};
use lazy_static::lazy_static;
use x86_64::{
	addr::{PhysAddr, VirtAddr},
	registers::control::{Cr3, Cr3Flags},
	structures::paging::{
		mapper::{Mapper, OffsetPageTable, Translate},
		page::PageRangeInclusive,
		page_table::{PageTableEntry, PageTableFlags},
		FrameAllocator, FrameDeallocator, PageTable, PhysFrame, Size1GiB, Size2MiB, Size4KiB,
	},
};

lazy_static! {
	static ref KERNEL_CR3: (PhysFrame, Cr3Flags) = Cr3::read();
}

/// Virtual address that the entire physical memory is mapped starting from.
const PHYSICAL_MAPPING_OFFSET: u64 = 0xFFFFC00000000000;

/// Map the given range of pages, to anywhere in the physical memory, on the current page table. Allocate
/// frames for them.
pub fn map_in_current(range: PageRangeInclusive, flags: PageTableFlags) {
	let table = get_current_page_table();
	map(range, table, flags);
}

/// Map the given range of pages, to anywhere in the physical memory, on the given page table. Allocate
/// frames for them.
pub fn map(range: PageRangeInclusive, table: &mut PageTable, flags: PageTableFlags) {
	let mut offset_table: OffsetPageTable;
	unsafe {
		offset_table = get_offset_page_table(table);
	}
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

/// Set the current page table.
/// # Safety
/// This is extremeley unsafe, for many reasons.
///  - changing page tables can mess with memory safety.
///  - The page table must live for as long as it is used
pub unsafe fn set_page_table(page_table: &PageTable) {
	let flags = Cr3Flags::empty();

	let virt_addr = VirtAddr::from_ptr(page_table);
	// let page = Page::containing_address(addr);

	let kernel_table = get_offset_page_table(get_kernel_page_table());

	// let frame = kernel_table
	// .translate_page(page)
	// .expect("Page table not mapped in kernel page table");
	let phys_addr = kernel_table
		.translate_addr(virt_addr)
		.expect("Page table not mapped in kernel page table");
	let frame = PhysFrame::containing_address(phys_addr);

	x86_64::registers::control::Cr3::write(frame, flags);
	// NOTE chaing cr3 flushes the tlb, no need to flush it manually
}

/// Set the current page table to the kernel
/// # Safety
/// Because the kernel page table is static we don't need to worry about it living long enough.
/// We do still need to worry about messing with memory safety
///  - changing page tables can mess with memory safety.
///
pub unsafe fn set_page_table_to_kernel() {
	// set_page_table(get_kernel_page_table()); // doesn't work because the kernel referance I use is mapped in the full mapping, not individually
	Cr3::write(KERNEL_CR3.0, KERNEL_CR3.1);
}

/// Set up paging. Clean up the page table created by the bootloader.
pub fn setup() {
	let table = get_current_page_table();

	unsafe {
		wipe_lower_half(table);
	}
}

/// Page table for a user process
#[derive(Debug)]
pub struct UserPageTable(pub Box<PageTable>);

impl Drop for UserPageTable {
	fn drop(&mut self) {
		unsafe {
			wipe_lower_half(&mut self.0);
		}
	}
}

/// Get a new userspace pagetable
pub fn get_new_user_table() -> UserPageTable {
	let user_table: PageTable = get_kernel_page_table().clone();
	UserPageTable(Box::new(user_table))
}

/// Wipes the lower half of a page table. Returns all physical frames that were mapped to in that
/// region to the [buddy allocator](buddy::BuddyAllocator), so that they can be allocated again.
unsafe fn wipe_lower_half(table: &mut PageTable) {
	for entry in table.iter_mut().take(256).filter(|entry| !entry.is_unused()) {
		wipe_recursive(entry, 4);
	}
}

/// Recursively wipes the page table from a given entry. Returns all physical frames that were mapped to in that
/// region to the [buddy allocator](buddy::BuddyAllocator), so that they can be allocated again.
unsafe fn wipe_recursive(entry: &mut PageTableEntry, depth: usize) {
	let sub_table = get_sub_table_mut(entry);
	if depth > 1 {
		match sub_table {
			Err(SubPageError::HugePage) => match depth {
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
			},
			Ok(table) => {
				for sub_entry in table.iter_mut().filter(|entry| !entry.is_unused()) {
					wipe_recursive(sub_entry, depth - 1);
				}
				buddy::ALLOCATOR
					.lock()
					.deallocate_frame(PhysFrame::<Size4KiB>::from_start_address(entry.addr()).unwrap());
			}
			Err(SubPageError::EntryUnused) => {
				unreachable!("Tried to wipe entry that is unused");
			}
		}
	} else {
		buddy::ALLOCATOR
			.lock()
			.deallocate_frame(PhysFrame::<Size4KiB>::from_start_address(entry.addr()).unwrap());
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

fn page_table_ref_from_cr3(cr3: (PhysFrame, Cr3Flags)) -> &'static mut PageTable {
	let phys_addr = cr3.0.start_address();
	unsafe { get_page_table_by_addr(phys_addr) }
}

/// Returns a reference (with static lifetime) to the current top level page table.
pub fn get_current_page_table() -> &'static mut PageTable {
	page_table_ref_from_cr3(Cr3::read())
}

/// Get the kernels page table
fn get_kernel_page_table() -> &'static mut PageTable {
	page_table_ref_from_cr3(*KERNEL_CR3)
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
	OffsetPageTable::new(page_table, offset)
}

/// Get a reference to the page table at a certain physical address.
/// ## Safety
/// This function is unsafe because it will read the data at whatever physical address you give it.
/// Make sure that this is the physical address of a page table.
unsafe fn get_page_table_by_addr(addr: PhysAddr) -> &'static mut PageTable {
	let virt_addr = phys_to_virt(addr);
	let table_ptr = virt_addr.as_mut_ptr();
	let page_table: &'static mut PageTable;
	page_table = &mut *table_ptr;
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
