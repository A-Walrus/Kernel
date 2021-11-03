use crate::serial_println;
use x86_64::{
	addr::{PhysAddr, VirtAddr},
	registers::control::Cr3,
	structures::paging::{
		mapper::{MappedPageTable, OffsetPageTable},
		page_table::PageTableFlags,
		PageTable,
	},
};

/// Virtual address that the entire physical memory is mapped starting from.
const PHYSICAL_MAPPING_OFFSET: u64 = 0xFFFFC00000000000;

/// Translate physical address to virtual address by adding constant [PHYSICAL_MAPPING_OFFSET].
fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
	VirtAddr::new(phys.as_u64() + PHYSICAL_MAPPING_OFFSET)
}

/// Returns a reference (with static lifetime) to the current top level page table.
pub fn get_current_page_table() -> &'static PageTable {
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
unsafe fn get_offset_page_table(page_table: &mut PageTable) -> OffsetPageTable {
	let offset = VirtAddr::new(PHYSICAL_MAPPING_OFFSET);
	unsafe { OffsetPageTable::new(page_table, offset) }
}

/// Get a reference to the page table at a certain physical address.
/// ## Safety
/// This function is unsafe because it will read the data at whatever physical address you give it.
/// Make sure that this is the physical address of a page table.
unsafe fn get_page_table_by_addr(addr: PhysAddr) -> &'static PageTable {
	let virt_addr = phys_to_virt(addr);
	let table_ptr = virt_addr.as_ptr();
	let page_table: &'static PageTable;
	unsafe {
		page_table = &*table_ptr;
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
				let sub_table = get_sub_table(page_table, i);
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
	NotAPageTable,
}

/// Gets the sub table at a certain index in a page table, where `0 ≤ index < 512`. If the entry is
/// unused, or is a huge page and not a page table, an error will be returned.
pub fn get_sub_table<'a>(page_table: &'a PageTable, index: usize) -> Result<&'a PageTable, SubPageError> {
	let entry = &page_table[index];
	if entry.is_unused() {
		Err(SubPageError::EntryUnused)
	} else if entry.flags().contains(PageTableFlags::HUGE_PAGE) {
		Err(SubPageError::NotAPageTable)
	} else {
		let phys_addr = entry.addr();
		Ok(unsafe { get_page_table_by_addr(phys_addr) })
	}
}
