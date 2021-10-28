use crate::serial_println;
use x86_64::{
	addr::{PhysAddr, VirtAddr},
	registers::control::Cr3,
	structures::paging::{mapper::MappedPageTable, PageTable},
};
const PHYSICAL_MAPPING_OFFSET: u64 = 0xFFFFC00000000000;

fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
	VirtAddr::new(phys.as_u64() + PHYSICAL_MAPPING_OFFSET)
}

pub fn get_page_table() -> &'static PageTable {
	let (phys_frame, _flags) = Cr3::read();
	let phys_addr = phys_frame.start_address();
	let virt_addr = phys_to_virt(phys_addr);
	let table_ptr = virt_addr.as_ptr();
	let page_table: &'static PageTable;
	unsafe {
		page_table = &*table_ptr;
	}
	page_table
}

pub fn print_table(page_table: &'static PageTable) {
	for (i, entry) in page_table.iter().enumerate() {
		if !entry.is_unused() {
			serial_println!("L4 Entry {}: {:?}", i, entry);
		}
	}
}
