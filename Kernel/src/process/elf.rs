use elf_rs::{self, Elf, ElfFile, ProgramType};

use x86_64::{
	addr::VirtAddr,
	structures::paging::{
		page::{Page, PageRangeInclusive},
		PageTableFlags, Size4KiB,
	},
};

use crate::{
	cpu::syscalls,
	fs::ext2::{self, Ext2Err},
	mem::paging,
};

/// Error relating to reading, parsing, loading, or executing an ELF executable file.
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

/// test elf parsing and mapping
pub fn test() -> Result<(), ElfErr> {
	let file_data = ext2::read_file("/bin/shell")?;
	let elf = Elf::from_bytes(&file_data).expect("failed to parse elf");
	let elf64 = match elf {
		Elf::Elf64(elf) => elf,
		_ => return Err(ElfErr::Elf32),
	};

	for header in elf64.program_header_iter() {
		serial_println!("{:?}", header);
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

			paging::map_in_current(range, flags);

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

	// TODO actually figure out where to put stack

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
	Ok(())
}
