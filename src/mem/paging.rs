use crate::serial_println;
use x86_64::{
	addr::{PhysAddr, VirtAddr},
	registers::control::Cr3,
	structures::paging::{mapper::MappedPageTable, PageTable},
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
pub unsafe fn get_page_table() -> &'static PageTable {
	let (phys_frame, _flags) = Cr3::read(); // CR3 register stores location of page table (and some flags)
	let phys_addr = phys_frame.start_address();
	let virt_addr = phys_to_virt(phys_addr);
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
