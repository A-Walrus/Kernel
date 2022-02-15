use core::mem::size_of;

use super::partitions;
use crate::{
	drivers::ahci::{
		disk::{BlockDevice, SectorReader},
		Sector, SECTOR_SIZE,
	},
	mem::heap::UBox,
};
use spin::Mutex;

fn undo_log_minus_10(num: u32) -> usize {
	1 << (num + 10)
}

#[repr(C)]
#[derive(Debug)]
struct SuperBlock {
	inodes: u32,
	blocks: u32,
	reserved_blocks: u32,
	unallocated_blocks: u32,
	unallocated_inodes: u32,
	superblock_block_num: u32,
	log_block_size_minus_10: u32,
	log_fragment_size_minus_10: u32,
	blocks_in_blockgroup: u32, // IMPORTANT
	fragments_in_blockgroup: u32,
	inodes_in_blockgroup: u32,
	last_mount_time: u32,
	last_written_time: u32,
	mounts_since_consistency_check: u16,
	mounts_allowed_before_consistency_check: u16,
	signature: u16, // should be 0xef53
	fs_state: u16,
	error_handling: u16,
	version_minor: u16,
	time_of_last_consistency_check: u32,
	interval_between_consistency_checks: u32,
	creator_os_id: u32,
	version_major: u32,
	user_id_reserved: u16,
	group_id_reserved: u16,
	first_non_reserved_inode: u32,
	inode_size: u16,
	this_block_group: u16,
	optional_features: u32,
	required_features: u32,
	write_features: u32,
	fs_id: [u8; 16],
	name: [u8; 16],
	last_mount_path: [u8; 64],
	compression_algorithms: u32,
	num_preallocate_blocks_file: u8,
	num_preallocate_blocks_dir: u8,
	_unused0: u16,
	journal_id: [u8; 16],
	journal_inode: u32,
	journal_device: u32,
	head_of_orphan_inode_list: u32,
	// _unused1: [u8; 1024 - 236],
}

impl SuperBlock {
	fn check_signature(&self) -> Result<(), ()> {
		if self.signature == 61267 {
			Ok(())
		} else {
			Err(())
		}
	}

	fn block_size(&self) -> usize {
		undo_log_minus_10(self.log_block_size_minus_10)
	}

	fn sectors_per_block(&self) -> usize {
		self.block_size() / 512
	}

	fn get_inode_blockgroup(&self, inode: u32) -> u32 {
		(inode - 1) / self.inodes_in_blockgroup
	}

	fn inode_index_in_blockgroup(&self, inode: u32) -> u32 {
		(inode - 1) % self.inodes_in_blockgroup
	}

	fn block_group_start_block(&self, group: u32) -> u32 {
		let start = if self.log_block_size_minus_10 == 0 { 2 } else { 1 };
		group * self.blocks_in_blockgroup + start
	}

	fn block_to_sector(&self, block: u32) -> usize {
		self.sectors_per_block() * (block as usize)
	}
}

#[repr(C)]
#[derive(Debug)]
struct BlockGroupDescriptor {
	block_usage_bitmap_address: u32,
	unode_usage_bitmap_address: u32,
	inode_table_starting_address: u32,
	unallocated_blocks_in_group: u16,
	unallocated_inodes_in_group: u16,
	dirs_in_group: u16,
	// _unused: [u8; 32 - 18],
}

#[repr(C)]
#[derive(Debug)]
struct InodeData {
	type_and_permissions: u16,
	user_id: u16,
	size_lower: u32,
	last_access_time: u32,
	creation_time: u32,
	last_modification_time: u32,
	deletion_time: u32,
	group_id: u16,
	hard_link_count: u16,
	sectors_in_user: u32, // Disk sectors (512b), Not ext2 Blocks (1K+)
	flags: u32,
	os_specific_val1: u32,
	direct_block_pointers: [u32; 12],
	singly_indirect_pointer: u32,
	doubly_indirect_pointer: u32,
	triply_indirect_pointer: u32,
	generation_number: u32,
	file_acl: u32,
	size_upper_or_directory_acl: u32,
	fragment_block_address: u32,
	os_specific_val2: [u8; 12],
}

#[repr(C)]
#[derive(Debug)]
struct DirectoryEntry {
	inode: u32,
	total_entry_size: u16,
	name_length_low: u8,
	type_indicator: u8,
	// name: () // TODO figure out how to represent the entries (not a fixed size 😡 ), and how to
	// represent a "list" of them, probably add iterator interface for convinience
}

/// Temporary entry point
pub fn entry() {
	let partition = Mutex::new(partitions::get_ext2_partition().unwrap());

	let mut reader = SectorReader::new(2, 0, &partition);
	let super_block: SuperBlock = unsafe { reader.read_type() };
	serial_println!("");
	serial_println!("{:?}", super_block);

	serial_println!("");
	super_block.check_signature().expect("Invalid ext signature!");
	serial_println!("Block size: {}", super_block.block_size());
	serial_println!("Inodes in group: {}", super_block.inodes_in_blockgroup);
	serial_println!("Size of inodes:  {}", super_block.inode_size);

	serial_println!("");

	let inode = 11; //root = 2, alice =11
	reader.move_to(
		super_block.block_to_sector(super_block.block_group_start_block(super_block.get_inode_blockgroup(inode))),
		0,
	);

	let block_group_desc: BlockGroupDescriptor = unsafe { reader.read_type() };
	serial_println!("{:?}", block_group_desc);

	serial_println!("");

	let inodes_per_sector = SECTOR_SIZE / super_block.inode_size as usize;
	let index = super_block.inode_index_in_blockgroup(inode);
	let containing_sector = ((index * super_block.inode_size as u32) as usize / SECTOR_SIZE) as usize;
	let offset = (index as usize % inodes_per_sector) * super_block.inode_size as usize;

	reader.move_to(
		super_block.block_to_sector(block_group_desc.inode_table_starting_address) + containing_sector,
		offset,
	);
	let inode: InodeData = unsafe { reader.read_type() };
	serial_println!("{:?}", inode);

	serial_println!("");
	// let file_reader = FileReader::new(inode, &super_block, &partition);
	use core::str;

	reader.move_to(super_block.block_to_sector(inode.direct_block_pointers[0]), 0);
	let data: Sector = unsafe { reader.read_type() };
	let string = str::from_utf8(&data).expect("String not utf8");
	serial_print!("{}", string);

	reader.move_to(1 + super_block.block_to_sector(inode.direct_block_pointers[0]), 0);
	let data: Sector = unsafe { reader.read_type() };
	let string = str::from_utf8(&data).expect("String not utf8");
	serial_print!("{}", string);

	reader.move_to(2 + super_block.block_to_sector(inode.direct_block_pointers[0]), 0);
	let data: Sector = unsafe { reader.read_type() };
	let string = str::from_utf8(&data).expect("String not utf8");
	serial_print!("{}", string);
}

// struct FileReader<'a> {
// 	inode: InodeData,
// 	reader: SectorReader<'a>,
// 	sectors_per_block: usize,
// }

// impl<'a> FileReader<'a> {
// 	fn new(inode: InodeData, super_block: &SuperBlock, block_device: &'a Mutex<dyn BlockDevice>) -> Self {
// 		let sectors_per_block = super_block.sectors_per_block();
// 		Self {
// 			inode,
// 			sectors_per_block: sectors_per_block,
// 			reader: SectorReader::new(
// 				sectors_per_block * inode.direct_block_pointers[0] as usize,
// 				0,
// 				block_device,
// 			),
// 		}
// 	}
// }

// struct DirectoryIter<'a> {
// 	reader: SectorReader<'a>,
// }

// impl<'a> Iterator for DirectoryIter<'a> {
// 	type Item = DirectoryEntry;

// 	fn next(&mut self) -> Option<Self::Item> {
// 		let entry: DirectoryEntry = unsafe { self.reader.read_type() };
// 	}
// }
