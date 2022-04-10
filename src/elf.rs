use core::borrow::Borrow;

use elf_rs::{self, Elf, ElfFile, ProgramHeaderFlags, ProgramType};

use x86_64::{
	addr::VirtAddr,
	structures::paging::{
		page::{Page, PageRangeInclusive},
		PageTableFlags, Size4KiB,
	},
};

use crate::{cpu::syscalls, fs::ext2, mem::paging};

/// test elf parsing and mapping
pub fn test() {
	let file_data = ext2::read_bin();
	let elf = Elf::from_bytes(&file_data).expect("failed to parse elf");
	let elf64 = match elf {
		Elf::Elf64(elf) => elf,
		_ => panic!("got Elf32, expected Elf64"),
	};
	for header in elf64.program_header_iter() {
		serial_println!("{:?}", header);
		if header.ph_type() == ProgramType::LOAD {
			let addr = VirtAddr::new(header.vaddr());
			let size = header.memsz();
			let start = header.offset() as usize;
			let end = start + header.filesz() as usize;
			let data: &[u8] = &file_data[start..end];

			let range = PageRangeInclusive::<Size4KiB> {
				start: Page::containing_address(addr),
				end: Page::containing_address(addr + size),
			};
			let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;

			paging::map_in_current(range, flags);

			unsafe {
				let target: *mut u8 = addr.as_mut_ptr();
				target.copy_from(data.as_ptr(), data.len());
			}
			// TODO pad zeroes
		}
	}
	const STACK_SIZE: u64 = 0x800000; // 8MiB
	const STACK_TOP: u64 = 0x1000000;

	let range = PageRangeInclusive::<Size4KiB> {
		start: Page::containing_address(VirtAddr::new(STACK_TOP - STACK_SIZE)),
		end: Page::containing_address(VirtAddr::new(STACK_TOP)),
	};
	let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;

	paging::map_in_current(range, flags);

	unsafe {
		syscalls::go_to_ring3(
			VirtAddr::new(elf64.elf_header().entry_point()),
			VirtAddr::new(STACK_TOP),
		);
	}
}
