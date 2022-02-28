use super::partitions;
use crate::{
	drivers::ahci::{
		disk::{BlockDevice, BlockReader},
		Sector, SECTOR_SIZE,
	},
	mem::heap::UBox,
	util::io::*,
};
use alloc::{boxed::Box, collections::VecDeque, str, string::String, vec::Vec};
use core::{cmp::min, mem::size_of, ptr::slice_from_raw_parts};
use spin::Mutex;

fn undo_log_minus_10(num: u32) -> usize {
	1 << (num + 10)
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
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

	fn inode_blockgroup(&self, inode: u32) -> u32 {
		(inode - 1) / self.inodes_in_blockgroup
	}

	fn inode_index_in_blockgroup(&self, inode: u32) -> u32 {
		(inode - 1) % self.inodes_in_blockgroup
	}

	fn block_group_start_block(&self, group: u32) -> u32 {
		let start = if self.log_block_size_minus_10 == 0 { 2 } else { 1 };
		serial_println!("Start {}", start);
		(group * self.blocks_in_blockgroup) + start
	}

	fn block_to_sector(&self, block: u32) -> usize {
		self.sectors_per_block() * (block as usize)
	}
}

#[repr(C)]
#[derive(Debug)]
struct BlockGroupDescriptor {
	block_bitmap_addr: u32,
	inode_bitmap_addr: u32,
	inode_table_starting_address: u32,
	unallocated_blocks: u16,
	unallocated_inodes: u16,
	dirs_in_group: u16,
	_unused: [u8; 32 - 18],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
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
}

/// Temporary entry point
pub fn entry() {
	let partition = Mutex::new(partitions::get_ext2_partition().unwrap());

	let mut sector_reader = BlockReader::new(2, 1, 0, &partition);
	let super_block: SuperBlock = unsafe { sector_reader.read_type().unwrap() };
	serial_println!("");
	serial_println!("{:?}", super_block);

	serial_println!("");
	super_block.check_signature().expect("Invalid ext signature!");
	serial_println!("Block size: {}", super_block.block_size());
	serial_println!("Inodes in group: {}", super_block.inodes_in_blockgroup);
	serial_println!("Blocks in group: {}", super_block.blocks_in_blockgroup);
	serial_println!("Size of inodes:  {}", super_block.inode_size);

	let path = "/foo/file.txt";

	let alice_inode = find_path_inode(path, &super_block, &partition).unwrap();
	serial_println!("{:?}", alice_inode);

	let mut file_reader = FileReader::new(alice_inode, &super_block, &partition);
	let mut data = Vec::new();
	file_reader.read_to_end(&mut data);
	let string = String::from_utf8(data);
	serial_println!("{}", string.unwrap());

	// let directory_iter = DirectoryIter { reader: file_reader };
	// for item in directory_iter {
	// 	serial_println!("{:?}", item);
	// }
	// remove_inode(11, &super_block, &partition);
}

fn find_path_inode(path: &str, super_block: &SuperBlock, device: &Mutex<dyn BlockDevice>) -> Option<u32> {
	let mut split = path.split("/");

	// Get rid of empty string before the first /
	split.next();
	// Root Inode
	let mut inode = 2;
	for name in split {
		let file_reader = FileReader::new(inode, super_block, device);
		let mut directory_iter = DirectoryIter { reader: file_reader };
		let result = directory_iter.find(|(entry, entryname)| name == entryname);
		match result {
			None => return None,
			Some((entry, _)) => {
				inode = entry.inode;
			}
		}
	}

	Some(inode)
}

fn get_inode_data<'a>(
	inode: u32,
	super_block: &'a SuperBlock,
	block_device: &'a Mutex<dyn BlockDevice>,
) -> (BlockGroupDescriptor, InodeData, BlockReader<'a>) {
	let group = super_block.inode_blockgroup(inode);
	let mut block_reader = BlockReader::new(
		super_block.block_group_start_block(group) as usize,
		super_block.sectors_per_block(),
		0,
		block_device,
	);

	block_reader.seek_forward(group as usize * size_of::<BlockGroupDescriptor>());

	let block_group_desc: BlockGroupDescriptor = unsafe { block_reader.read_type().unwrap() };
	serial_println!("{:?}", block_group_desc);
	serial_println!("");
	block_reader.move_to_block(block_group_desc.inode_table_starting_address as usize);
	block_reader.seek_forward(super_block.inode_index_in_blockgroup(inode) as usize * super_block.inode_size as usize);
	let inode_data: InodeData = unsafe { block_reader.read_type().unwrap() };
	(block_group_desc, inode_data, block_reader)
}

fn remove_inode(inode: u32, super_block: &SuperBlock, block_device: &Mutex<dyn BlockDevice>) {
	let (block_group_desc, inode_data, mut block_reader) = get_inode_data(inode, super_block, block_device);
	serial_println!("{:?}", inode_data);
	let block_iter: InodeBlockIter<true> = InodeBlockIter::new(inode_data, block_reader.clone());

	// Mark inode free in inode bitmap
	{
		let inode_bitmap = block_reader.read_block(block_group_desc.inode_bitmap_addr);
		let inode_index_in_blockgroup = super_block.inode_index_in_blockgroup(inode);
		let index_in_slice = inode_index_in_blockgroup as usize / 8;
		// zero the corresponding bit
		inode_bitmap[index_in_slice] &= !((1 << (inode_index_in_blockgroup % 8)) as u8);
		block_reader.write_current_block();
	}

	let group = super_block.inode_blockgroup(inode);
	let start_of_block_group = super_block.block_group_start_block(group);
	let mut freed_blocks = 0;
	// Mark blocks free in block bitmap
	{
		let block_bitmap = block_reader.read_block(block_group_desc.block_bitmap_addr);
		// serial_println!("Start of block group {}", start_of_block_group);
		for block in block_iter {
			// TODO figure out if and why +1 is correct
			let block_index = block + 1 - start_of_block_group;

			assert!(
				block_index < super_block.blocks_in_blockgroup,
				"Trying to free block in another group"
			);

			let index_in_slice = block_index as usize / 8;

			assert!(
				(block_bitmap[index_in_slice] >> (block_index % 8)) % 2 == 1,
				"Freeing free block? at block {}: {}",
				freed_blocks,
				block
			);

			// zero the corresponding bit
			block_bitmap[index_in_slice] &= !((1 << (block_index % 8)) as u8);
			freed_blocks += 1;
		}
		block_reader.write_current_block();
		serial_println!("Blocks freed: {}", freed_blocks);
	}

	// fix block group descriptor (free inode count, free block count)
	block_reader.move_to_block(start_of_block_group as usize);
	block_reader.seek_forward(group as usize * size_of::<BlockGroupDescriptor>());
	let mut new_descriptor = block_group_desc;
	new_descriptor.unallocated_blocks += freed_blocks as u16;
	new_descriptor.unallocated_inodes += 1;
	block_reader.write_type(&new_descriptor);
	block_reader.flush();

	// fix superblock (free inode count, free block count)
	let mut new_superblock: SuperBlock = *super_block;
	new_superblock.unallocated_inodes += 1;
	new_superblock.unallocated_blocks += freed_blocks;
	let mut sector_reader = BlockReader::new(2, 1, 0, block_device);
	sector_reader.write_type(&new_superblock);
	sector_reader.flush();
}

struct InodeBlockIter<'a, const WITHPARENTS: bool> {
	blocks: [VecDeque<u32>; 4],
	inode_data: InodeData,
	reader: BlockReader<'a>,
}

impl<'a, const WITHPARENTS: bool> InodeBlockIter<'a, WITHPARENTS> {
	fn get_next_block_of(&mut self, level: usize) -> Option<u32> {
		match self.blocks[level].pop_front() {
			Some(0) => None,
			Some(block) => Some(block),
			None => {
				let parent = self.get_next_block_of(level + 1);
				match parent {
					Some(block) => {
						let slice = self.reader.read_block(block);
						let sub_blocks;
						unsafe {
							sub_blocks = slice_from_raw_parts(slice.as_ptr() as *const u32, slice.len() / 4)
								.as_ref()
								.unwrap();
							self.blocks[level].extend(sub_blocks);
						}
						if WITHPARENTS {
							parent
						} else {
							let new_block = self.blocks[level].pop_front().unwrap();
							assert_ne!(new_block, 0);
							Some(new_block)
						}
					}
					None => None,
				}
			}
		}
	}

	fn new(inode_data: InodeData, reader: BlockReader<'a>) -> Self {
		let direct = inode_data.direct_block_pointers;
		let indirect = inode_data.singly_indirect_pointer;
		let double_indirect = inode_data.doubly_indirect_pointer;
		let triple_indirect = inode_data.triply_indirect_pointer;
		let direct_blocks = VecDeque::from(direct);
		let singly_indirect_blocks = if indirect == 0 {
			VecDeque::new()
		} else {
			VecDeque::from([indirect])
		};
		let doubly_indirect_blocks = if indirect == 0 {
			VecDeque::new()
		} else {
			VecDeque::from([double_indirect])
		};
		let tripy_indirect_blocks = if indirect == 0 {
			VecDeque::new()
		} else {
			VecDeque::from([triple_indirect])
		};
		Self {
			inode_data,
			blocks: [
				direct_blocks,
				singly_indirect_blocks,
				doubly_indirect_blocks,
				tripy_indirect_blocks,
			],
			reader,
		}
	}
}

impl<const WITHPARENTS: bool> Iterator for InodeBlockIter<'_, WITHPARENTS> {
	type Item = u32;
	fn next(&mut self) -> Option<Self::Item> {
		self.get_next_block_of(0)
	}
}

struct FileReader<'a> {
	inode_data: InodeData,
	reader: BlockReader<'a>,
	position: usize,
	inode_block_iter: InodeBlockIter<'a, false>,
}

impl<'a> FileReader<'a> {
	fn new(inode: u32, super_block: &'a SuperBlock, block_device: &'a Mutex<dyn BlockDevice>) -> Self {
		let (_, inode_data, block_reader) = get_inode_data(inode, super_block, block_device);
		serial_println!("{:?}", inode_data);

		let clone = block_reader.clone();
		Self {
			inode_data,
			reader: block_reader,
			position: 0,
			inode_block_iter: InodeBlockIter::new(inode_data, clone),
		}
	}
}
impl<'a> Read for FileReader<'a> {
	fn read(&mut self, mut buf: &mut [u8]) -> Result<usize, IOError> {
		let to_read = min(buf.len(), self.inode_data.size_lower as usize - self.position);
		let mut left_to_read = to_read;

		let block_size = self.reader.slice().len();
		let mut block = self.reader.slice();
		while left_to_read > 0 {
			let offset_in_block = self.position % block_size;
			let to_read_from_block = min(left_to_read, block_size - offset_in_block);
			if offset_in_block == 0 {
				let next_block = self.inode_block_iter.next().unwrap();
				block = self.reader.read_block(next_block);
			}
			buf[..to_read_from_block].copy_from_slice(&block[offset_in_block..offset_in_block + to_read_from_block]);
			buf = &mut buf[to_read_from_block..];
			left_to_read -= to_read_from_block;
			self.position += to_read_from_block;
		}
		Ok(to_read)
	}
}

struct DirectoryIter<'a> {
	reader: FileReader<'a>,
}

impl<'a> Iterator for DirectoryIter<'a> {
	type Item = (DirectoryEntry, String);

	fn next(&mut self) -> Option<Self::Item> {
		let result = unsafe { self.reader.read_type::<DirectoryEntry>() };
		if let Ok(entry) = result {
			let len = entry.total_entry_size as usize - size_of::<DirectoryEntry>();
			let mut name = vec![0u8; len];
			if self.reader.read(&mut name).is_err() {
				return None;
			}
			name.truncate(entry.name_length_low as usize);
			Some((entry, String::from_utf8(name).unwrap()))
		} else {
			None
		}
	}
}
