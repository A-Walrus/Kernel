use crate::serial_println;
use x86_64::{
	addr::{PhysAddr, VirtAddr},
	registers::control::Cr3,
	structures::paging::{mapper::MappedPageTable, page_table::PageTableFlags, PageTable},
};
/// Virtual address that the entire physical memory is mapped starting from.
const PHYSICAL_MAPPING_OFFSET: u64 = 0xFFFFC00000000000;

/// Translate physical address to virtual address by adding constant [PHYSICAL_MAPPING_OFFSET].
fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
	VirtAddr::new(phys.as_u64() + PHYSICAL_MAPPING_OFFSET)
}

/// Returns a reference (with static lifetime) to the current top level page table.
/// ## Safety
/// The caller must insure:
/// * The full physical memory is mapped at [PHYSICAL_MAPPING_OFFSET] (otherwise you wouldn't be
/// reading the page table).
/// * The reference isn't used after [Cr3] has been modified (The reference would be pointing to
/// garbage).
pub unsafe fn get_current_page_table() -> &'static PageTable {
	let (phys_frame, _flags) = Cr3::read(); // CR3 register stores location of page table (and some flags)
	let phys_addr = phys_frame.start_address();
	unsafe { get_page_table_by_addr(phys_addr) }
}

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
