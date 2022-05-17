use core::{
	mem::{align_of, size_of},
	slice::from_raw_parts_mut,
	str::from_utf8,
};
use elf_rs::{self, Elf, ElfFile, Error, ProgramType};

use x86_64::{
	addr::VirtAddr,
	registers::control::Cr3,
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
	/// Error related to the structure of the elf file
	Elf(Error),
}

impl From<Error> for ElfErr {
	fn from(e: Error) -> Self {
		ElfErr::Elf(e)
	}
}

impl From<Ext2Err> for ElfErr {
	fn from(e: Ext2Err) -> Self {
		ElfErr::Fs(e)
	}
}

/// Data after loading an elf process
#[derive(Debug, Copy, Clone)]
pub struct LoadData {
	/// Entrypoint
	pub entry: VirtAddr,
	/// Top of stack
	pub stack_top: VirtAddr,
	/// Number of arguements
	pub argc: usize,
	/// Pointer to arguemnt vector
	pub argv: VirtAddr,
}

/// load an ELF executable
// pub fn load_elf(path: &str, page_table: &mut PageTable, args: &[&str]) -> Result<(VirtAddr, VirtAddr), ElfErr> {
pub fn load_elf(path: &str, page_table: &mut PageTable, args: &[&str]) -> Result<LoadData, ElfErr> {
	serial_println!("Loading ELF {}", path);
	let file_data = ext2::read_file(path)?;
	let elf = Elf::from_bytes(&file_data)?;
	let elf64 = match elf {
		Elf::Elf64(elf) => elf,
		_ => return Err(ElfErr::Elf32),
	};

	let prev_table = Cr3::read();

	// Switch to user table
	unsafe {
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

	// Map stack
	const STACK_SIZE: u64 = 0x800000; // 8MiB
	const STACK_TOP: u64 = 0x0000800000000000 - 1; // top of userspace

	let range = PageRangeInclusive::<Size4KiB> {
		start: Page::containing_address(VirtAddr::new(STACK_TOP - STACK_SIZE)),
		end: Page::containing_address(VirtAddr::new(STACK_TOP)),
	};
	let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;

	paging::map(range, page_table, flags);

	let mut stack_top: usize = (STACK_TOP as usize) & !(align_of::<&str>() - 1);

	let len = args.len();
	let size = len * size_of::<&str>();
	let slice: &mut [&str] = unsafe { from_raw_parts_mut((stack_top - size) as *mut &str, len) };
	slice.copy_from_slice(args);
	stack_top -= size;

	for arg in slice.iter_mut() {
		let len = arg.len();
		let slice: &mut [u8] = unsafe { from_raw_parts_mut((stack_top - len) as *mut u8, len) };
		slice.copy_from_slice(arg.as_bytes());
		stack_top -= len;
		*arg = from_utf8(slice).unwrap();
	}

	// Map heap
	const HEAP_SIZE: u64 = 0x800000; // 8MiB
	const HEAP_START: u64 = 0x0000400000000000;

	let range = PageRangeInclusive::<Size4KiB> {
		start: Page::containing_address(VirtAddr::new(HEAP_START)),
		end: Page::containing_address(VirtAddr::new(HEAP_START + HEAP_SIZE)),
	};
	let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;

	paging::map(range, page_table, flags);

	// Switch back to original page table
	unsafe {
		Cr3::write(prev_table.0, prev_table.1);
	}

	let entry = elf64.elf_header().entry_point();
	Ok(LoadData {
		entry: VirtAddr::new(entry),
		stack_top: VirtAddr::new(stack_top as u64),
		argc: args.len(),
		argv: VirtAddr::from_ptr(slice.as_ptr()),
	})
}
