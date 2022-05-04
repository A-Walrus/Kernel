use elf_rs::{self, Elf, ElfFile, ProgramType};

use x86_64::{
	addr::VirtAddr,
	registers::control::Cr3Flags,
	structures::paging::{
		page::{Page, PageRangeInclusive},
		PageTable, PageTableFlags, Size4KiB,
	},
};

use crate::{
	fs::ext2::{self, Ext2Err},
	mem::paging,
};

/// Error relating to reading, parsing, loading, or executing an ELF executable file.
#[derive(Debug, Copy, Clone)]
pub enum ElfErr {
	/// Error related to the file system
	Fs(Ext2Err),
	/// Only 64 bit executables are supported
	Elf32,
}

impl From<Ext2Err> for ElfErr {
	fn from(e: Ext2Err) -> Self {
		ElfErr::Fs(e)
	}
}

/// load an ELF executable
pub fn load_elf(path: &str, page_table: &mut PageTable) -> Result<(VirtAddr, VirtAddr), ElfErr> {
	serial_println!("Loading ELF {}", path);
	let file_data = ext2::read_file(path)?;
	let elf = Elf::from_bytes(&file_data).expect("failed to parse elf");
	let elf64 = match elf {
		Elf::Elf64(elf) => elf,
		_ => return Err(ElfErr::Elf32),
	};

	unsafe {
		serial_println!("Switching to user table");
		paging::set_page_table(page_table);
	}

	for header in elf64.program_header_iter() {
		serial_println!(" - ELF Header: {:?}", header);
		if header.ph_type() == ProgramType::LOAD {
			let addr = VirtAddr::new(header.vaddr());
			let size = header.memsz() as usize;
			let start = header.offset() as usize;
			let filesz = header.filesz() as usize;
			let end = start + filesz;
			let data: &[u8] = &file_data[start..end];

			let range = PageRangeInclusive::<Size4KiB> {
				start: Page::containing_address(addr),
				end: Page::containing_address(addr + size),
			};
			let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;

			paging::map(range, page_table, flags);

			if header.memsz() > 0 {
				// If there is data, copy it in
				unsafe {
					let target: *mut u8 = addr.as_mut_ptr();
					target.copy_from(data.as_ptr(), data.len());

					// Pad zeroes (especially important for BSS)
					let padding_start = target.add(filesz);
					let padding_len = size - filesz;
					padding_start.write_bytes(0, padding_len);
				}
			}
		}
	}

	const STACK_SIZE: u64 = 0x800000; // 8MiB
	const STACK_TOP: u64 = 0x0000800000000000 - 1; // top of userspace

	let range = PageRangeInclusive::<Size4KiB> {
		start: Page::containing_address(VirtAddr::new(STACK_TOP - STACK_SIZE)),
		end: Page::containing_address(VirtAddr::new(STACK_TOP)),
	};
	let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;

	paging::map(range, page_table, flags);

	const HEAP_SIZE: u64 = 0x800000; // 8MiB
	const HEAP_START: u64 = 0x0000400000000000;

	let range = PageRangeInclusive::<Size4KiB> {
		start: Page::containing_address(VirtAddr::new(HEAP_START)),
		end: Page::containing_address(VirtAddr::new(HEAP_START + HEAP_SIZE)),
	};
	let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;

	paging::map(range, page_table, flags);

	serial_println!("Switching to kernel table");
	unsafe {
		paging::set_page_table_to_kernel();
	}
	serial_println!("Switched to kernel table");

	let entry = elf64.elf_header().entry_point();
	Ok((VirtAddr::new(entry), VirtAddr::new(STACK_TOP)))
}
