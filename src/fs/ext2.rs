use super::partitions;
use crate::{
	drivers::ahci::disk::{BlockDevice, BlockReader, Partition},
	util::io::*,
};
use alloc::{
	collections::VecDeque,
	str,
	string::{FromUtf8Error, String},
	vec::Vec,
};
use core::{
	cmp::min,
	mem::size_of,
	ops::{Index, IndexMut},
	ptr::slice_from_raw_parts,
};
use spin::Mutex;

static mut DEVICE: Option<Mutex<Partition>> = None;

static mut EXT: Option<Mutex<Ext2>> = None;

const ROOT_INODE: Inode = 2;

macro_rules! get_device {
	() => {
		unsafe { DEVICE.as_ref().unwrap() }
	};
}

macro_rules! get_ext {
	() => {
		unsafe { EXT.as_ref().unwrap() }
	};
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
/// Literal structure found on disk, the SuperBlock
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

	fn inode_blockgroup(&self, inode: Inode) -> Group {
		(inode - 1) / self.inodes_in_blockgroup
	}

	fn inode_index_in_blockgroup(&self, inode: Inode) -> u32 {
		(inode - 1) % self.inodes_in_blockgroup
	}

	fn block_group_start_block(&self, group: Group) -> Block {
		let start = if self.log_block_size_minus_10 == 0 { 2 } else { 1 };
		serial_println!("Start {}", start);
		(group * self.blocks_in_blockgroup) + start
	}

	fn num_blockgroups(&self) -> u32 {
		self.blocks / self.blocks_in_blockgroup
	}
}

#[repr(C)]
#[derive(Debug)]
/// Literal structure found on disk, the block group descriptor
struct BlockGroupDescriptor {
	block_bitmap_addr: Block,
	inode_bitmap_addr: Block,
	inode_table_addr: Block,
	unallocated_blocks: u16,
	unallocated_inodes: u16,
	dirs_in_group: u16,
	_unused: [u8; 32 - 18],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
/// Literal structure found on disk, the inode
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
	direct_block_pointers: [Block; 12],
	singly_indirect_pointer: Block,
	doubly_indirect_pointer: Block,
	triply_indirect_pointer: Block,
	generation_number: u32,
	file_acl: u32,
	size_upper_or_directory_acl: u32,
	fragment_block_address: u32,
	os_specific_val2: [u8; 12],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
/// Literal structure found on disk, the directory entry
struct DirectoryEntry {
	inode: Inode,
	total_entry_size: u16,
	name_length_low: u8,
	type_indicator: u8,
}

/// Some ext fields store their log minus 10, this undos that operation.
fn undo_log_minus_10(num: u32) -> usize {
	1 << (num + 10)
}

// Abstract land
// ------------------------------------------------------------------

/// Abstract custom struct representing an easy to work with directory
struct Directory {
	entries: Vec<Entry>,
}

impl Directory {
	fn read(reader: &mut FileReader) -> Result<Self, Ext2Err> {
		const ENTRY_SIZE: usize = size_of::<DirectoryEntry>();

		let mut entries = Vec::new();
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes)?;
		let mut slice = &*bytes;
		loop {
			if slice.len() == 0 {
				break;
			}
			let ptr = slice.as_ptr() as *const DirectoryEntry;
			let entry = unsafe { *ptr };
			slice = &slice[ENTRY_SIZE..];
			let name = String::from_utf8(Vec::from(&slice[..entry.name_length_low as usize]))?;
			entries.push(Entry { entry, name });
			slice = &slice[entry.total_entry_size as usize - ENTRY_SIZE..];
		}
		Ok(Self { entries })
	}

	fn write(&self, writer: FileReader) {
		for entry in &self.entries {}
		unimplemented!()
	}
}

/// Struct representing an entry in an abstract directory
#[derive(Clone, Debug)]
struct Entry {
	entry: DirectoryEntry,
	name: String,
}

impl Entry {
	fn set_name(&mut self, name: String) {
		self.entry.name_length_low = name.len() as u8;
		self.name = name;
	}
}

struct Ext2 {
	super_block: SuperBlock,
	block_groups: Vec<BlockGroup>,
}

impl Ext2 {
	fn get_inode_data(&self, inode: Inode) -> &InodeData {
		let group = self.super_block.inode_blockgroup(inode);
		let index_in_group = self.super_block.inode_index_in_blockgroup(inode);
		let group = &self.block_groups[group as usize];
		&group.inode_table[index_in_group]
	}

	fn get_inode_data_mut(&mut self, inode: Inode) -> &mut InodeData {
		let group = self.super_block.inode_blockgroup(inode);
		let index_in_group = self.super_block.inode_index_in_blockgroup(inode);
		let group = &mut self.block_groups[group as usize];
		&mut group.inode_table[index_in_group]
	}

	fn read_from_disk() -> Result<Self, Ext2Err> {
		let block_device = get_device!();

		let mut sector_reader = BlockReader::new(2, 1, 0, block_device);
		let super_block: SuperBlock = unsafe { sector_reader.read_type().unwrap() };

		let mut block_reader = BlockReader::new(0, super_block.sectors_per_block(), 0, block_device);

		let mut block_groups = Vec::new();
		for group_index in 0..super_block.num_blockgroups() {
			block_reader.move_to_block(super_block.block_group_start_block(group_index));
			let descriptor: BlockGroupDescriptor = unsafe { block_reader.read_type()? };

			let table_size = super_block.inodes_in_blockgroup as usize * super_block.inode_size as usize;
			let mut blocks = vec![0; table_size];
			block_reader.move_to_block(descriptor.inode_table_addr);
			block_reader.read(&mut blocks)?;

			let inode_table = InodeTable {
				inode_size: super_block.inode_size as usize,
				bytes: blocks,
			};

			let bytes = Vec::from(block_reader.read_block(descriptor.inode_bitmap_addr));
			let inode_bitmap = ExtBitMap { bytes };

			let bytes = Vec::from(block_reader.read_block(descriptor.block_bitmap_addr));
			let block_bitmap = ExtBitMap { bytes };

			block_groups.push(BlockGroup {
				descriptor,
				inode_table,
				inode_bitmap,
				block_bitmap,
				first_inode: (super_block.inodes_in_blockgroup * group_index) + 1,
				first_block: super_block.block_group_start_block(group_index),
			});
		}
		Ok(Ext2 {
			super_block,
			block_groups,
		})
	}

	fn write_to_disk(&self) -> Result<(), Ext2Err> {
		let block_device = get_device!();

		let mut sector_reader = BlockReader::new(2, 1, 0, block_device);
		sector_reader.write_type(&self.super_block)?;

		let mut block_reader = BlockReader::new(0, self.super_block.sectors_per_block(), 0, block_device);

		for group in &self.block_groups {
			block_reader.move_to_block(group.first_block);
			block_reader.write_type(&group.descriptor)?;

			block_reader.move_to_block(group.descriptor.inode_bitmap_addr);
			block_reader.write(&group.inode_bitmap.bytes)?;

			block_reader.move_to_block(group.descriptor.block_bitmap_addr);
			block_reader.write(&group.block_bitmap.bytes)?;

			block_reader.move_to_block(group.descriptor.inode_table_addr);
			block_reader.write(&group.inode_table.bytes)?;
		}

		Ok(())
	}

	fn get_free_block(&mut self) -> Result<Block, Ext2Err> {
		for block in self.block_groups.iter_mut() {
			let result = block.get_free_block();
			match result {
				Ok(block) => {
					self.super_block.unallocated_blocks -= 1;
					return Ok(block);
				}
				_ => {}
			}
		}
		Err(NoBlocks)
	}

	fn get_free_inode(&mut self) -> Result<Inode, Ext2Err> {
		for block in self.block_groups.iter_mut() {
			let result = block.get_free_inode();
			match result {
				Ok(inode) => {
					self.super_block.unallocated_inodes -= 1;
					return Ok(inode);
				}
				_ => {}
			}
		}
		Err(NoInodes)
	}

	fn free_block(&mut self, block: Block) -> Result<(), Ext2Err> {
		unimplemented!();
		Ok(())
	}

	fn free_inode(&mut self, block: Block) -> Result<(), Ext2Err> {
		unimplemented!();
		Ok(())
	}
}

struct BlockGroup {
	descriptor: BlockGroupDescriptor,
	inode_table: InodeTable,
	inode_bitmap: ExtBitMap,
	block_bitmap: ExtBitMap,
	first_inode: Inode,
	first_block: Block,
}

impl BlockGroup {
	/// Try to get a free inode, mark it as used
	fn get_free_inode(&mut self) -> Result<Inode, Ext2Err> {
		if self.descriptor.unallocated_inodes == 0 {
			Err(NoInodes)
		} else {
			self.descriptor.unallocated_inodes -= 1;
			let position = self.inode_bitmap.get_free().unwrap();
			self.inode_bitmap.set(position, Used);
			Ok(self.first_inode + (position as u32))
		}
	}

	/// Try to get a free block, mark it as used
	fn get_free_block(&mut self) -> Result<Block, Ext2Err> {
		if self.descriptor.unallocated_blocks == 0 {
			Err(NoBlocks)
		} else {
			self.descriptor.unallocated_blocks -= 1;
			let position = self.block_bitmap.get_free().unwrap();
			self.block_bitmap.set(position, Used);
			Ok(self.first_block + (position as u32))
		}
	}
}

struct ExtBitMap {
	bytes: Vec<u8>,
}

impl ExtBitMap {
	fn set(&mut self, place: usize, value: Bit) {
		let index = place / 8;
		let offset = place / 8;
		self.bytes[index] = match value {
			Free => self.bytes[index] & !(1 << offset),
			Used => self.bytes[index] | (1 << offset),
		}
	}

	fn get(&self, place: usize) -> Bit {
		let index = place / 8;
		let offset = place / 8;
		if self.bytes[index] >> offset % 2 == 0 {
			Free
		} else {
			Used
		}
	}

	/// Find a free bit, and mark it as used
	fn get_free(&mut self) -> Option<usize> {
		for (i, byte) in self.bytes.iter_mut().enumerate() {
			if *byte == u8::MAX {
				continue;
			}
			let mut val: u8 = *byte;
			for j in 0..8 {
				if val % 2 == 0 {
					// found
					*byte |= 1 << j;

					return Some(((i * 8) + j + 1) as usize);
				} else {
					val = val >> 1;
				}
			}
		}
		None
	}
}

fn unlink(path: &str) -> Result<(), Ext2Err> {
	let mut ext = get_ext!().lock();
	let device = get_device!();
	let index = path.rfind("/").unwrap();
	let folder_path = &path[..index];
	let file_name = &path[index + 1..];
	let dir_inode = path_to_inode(folder_path);

	// TODO Remove entry
	unimplemented!();

	let sectors_per_block = ext.super_block.sectors_per_block();
	let inode = path_to_inode(path)?;
	let inode_data = *ext.get_inode_data(inode);
	if inode_data.hard_link_count == 1 {
		ext.free_inode(inode)?;
		let mut b_reader = BlockReader::new(0, sectors_per_block, 0, device);
		let blocks = get_inode_blocks(inode_data, &mut b_reader, true);
		for block in blocks {
			ext.free_block(block)?;
		}
	} else {
		ext.get_inode_data_mut(inode).hard_link_count -= 1;
	}
	Ok(())
}

fn path_to_inode(path: &str) -> Result<Inode, Ext2Err> {
	if path.chars().nth(0) != Some('/') {
		return Err(NotAbsolute);
	}
	let mut split = path.split("/");

	// Get rid of empty string before the first /
	split.next();

	// Root Inode
	let mut inode: Inode = ROOT_INODE;

	for name in split {
		let mut file_reader = FileReader::new(inode);
		let directory = Directory::read(&mut file_reader)?;
		let result = directory.entries.iter().find(|entry| name == entry.name);
		match result {
			None => return Err(FileNotFound),
			Some(entry) => {
				inode = entry.entry.inode;
			}
		}
	}
	Ok(inode)
}

fn get_inode_blocks(inode: InodeData, b_reader: &mut BlockReader, with_parents: bool) -> Vec<Block> {
	let mut blocks = Vec::new();

	let result = inode.direct_block_pointers.iter().position(|val| *val == 0);
	match result {
		Some(index) => {
			blocks.extend_from_slice(&inode.direct_block_pointers[..index]);
		}
		None => {
			blocks.extend_from_slice(&inode.direct_block_pointers);
			if inode.singly_indirect_pointer == 0 {
				return blocks;
			}
			get_indirect_blocks(&mut blocks, b_reader, inode.singly_indirect_pointer, 1, with_parents);
			get_indirect_blocks(&mut blocks, b_reader, inode.doubly_indirect_pointer, 2, with_parents);
			get_indirect_blocks(&mut blocks, b_reader, inode.triply_indirect_pointer, 3, with_parents);
		}
	}

	blocks
}

fn get_indirect_blocks(
	blocks: &mut Vec<Block>,
	b_reader: &mut BlockReader,
	block: Block,
	indirectness: usize,
	with_parents: bool,
) {
	if block == 0 {
		return;
	} else {
		let slice = b_reader.read_block(block);

		let mut sub_blocks;
		unsafe {
			sub_blocks = slice_from_raw_parts(slice.as_ptr() as *const u32, slice.len() / 4)
				.as_ref()
				.unwrap();
		}
		let result = sub_blocks.iter().position(|val| *val == 0);
		if let Some(index) = result {
			sub_blocks = &sub_blocks[..index]
		}
		if indirectness > 1 {
			if with_parents {
				blocks.extend_from_slice(sub_blocks);
			}
			for block in sub_blocks {
				get_indirect_blocks(blocks, b_reader, *block, indirectness, with_parents);
			}
		} else {
			blocks.extend_from_slice(sub_blocks);
		}
	}
}

struct FileReader<'a> {
	inode: Inode,
	inode_data: InodeData,
	reader: BlockReader<'a>,
	position: usize,
	blocks: Vec<Block>,
}

impl<'a> FileReader<'a> {
	fn new(inode: u32) -> Self {
		let ext = get_ext!();
		let device = get_device!();
		let inode_data = *ext.lock().get_inode_data(inode);
		let super_block = ext.lock().super_block;

		let mut block_reader = BlockReader::new(0, super_block.sectors_per_block(), 0, device);
		let blocks = get_inode_blocks(inode_data, &mut block_reader, false);
		Self {
			inode,
			inode_data,
			reader: block_reader,
			position: 0,
			blocks,
		}
	}
}
impl<'a> Read for FileReader<'a> {
	fn read(&mut self, mut buf: &mut [u8]) -> Result<usize, IOError> {
		let to_read = min(buf.len(), self.inode_data.size_lower as usize - self.position);
		let mut left_to_read = to_read;

		let block_size = self.reader.slice().len();
		let mut block;
		while left_to_read > 0 {
			let offset_in_block = self.position % block_size;
			let to_read_from_block = min(left_to_read, block_size - offset_in_block);
			let next_block = self.blocks[self.position / block_size];
			block = self.reader.read_block(next_block);
			buf[..to_read_from_block].copy_from_slice(&block[offset_in_block..offset_in_block + to_read_from_block]);
			buf = &mut buf[to_read_from_block..];
			left_to_read -= to_read_from_block;
			self.position += to_read_from_block;
		}
		Ok(to_read)
	}
}

impl<'a> Seek for FileReader<'a> {
	fn seek(&mut self, pos: SeekFrom) -> Result<usize, IOError> {
		match pos {
			SeekFrom::Start(offset) => {
				self.position = offset;
				Ok(offset)
			}
			SeekFrom::Current(offset) => {
				self.position = (self.position as isize + offset) as usize;
				Ok(self.position)
			}
			SeekFrom::End(offset) => unimplemented!(),
		}
	}
}

impl<'a> Write for FileReader<'a> {
	fn write(&mut self, mut buf: &[u8]) -> Result<usize, IOError> {
		let mut added_blocks = Vec::new();

		let ext = get_ext!();
		let to_write = min(buf.len(), self.inode_data.size_lower as usize - self.position);
		let mut left_to_write = to_write;

		let block_size = self.reader.slice().len();
		let mut block;
		while left_to_write > 0 {
			let offset_in_block = self.position % block_size;
			let to_write_to_block = min(left_to_write, block_size - offset_in_block);

			// let next_block = self.blocks[self.position / block_size];
			let next_block = match self.blocks.get(self.position / block_size) {
				Some(block) => *block,
				None => {
					let free_block = ext.lock().get_free_block().map_err(|a| IOError::Other)?;
					added_blocks.push(free_block);

					free_block
				}
			};

			block = self.reader.read_block(next_block);
			// buf[..to_read_from_block].copy_from_slice(&block[offset_in_block..offset_in_block + to_read_from_block]);
			block[offset_in_block..offset_in_block + to_write_to_block].copy_from_slice(&buf[..to_write_to_block]);
			buf = &buf[to_write_to_block..];
			left_to_write -= to_write_to_block;
			self.position += to_write_to_block;
		}

		// TODO add added blocks to inode (with indirectness...)

		Ok(to_write)
	}

	fn flush(&mut self) -> Result<(), IOError> {
		self.reader.flush()
	}
}

impl<'a> Drop for FileReader<'a> {
	fn drop(&mut self) {
		let ext = get_ext!();
		*ext.lock().get_inode_data_mut(self.inode) = self.inode_data;
	}
}

use Bit::*;
#[derive(Copy, Clone, Debug)]
enum Bit {
	Free,
	Used,
}

struct InodeTable {
	bytes: Vec<u8>,
	inode_size: usize,
}

impl Index<Inode> for InodeTable {
	type Output = InodeData;
	fn index(&self, index: Inode) -> &Self::Output {
		let idx = index as usize;
		let u8_ref = &self.bytes[idx * self.inode_size];
		let ptr = u8_ref as *const u8 as *const InodeData;
		unsafe { &*ptr }
	}
}

impl IndexMut<Inode> for InodeTable {
	fn index_mut(&mut self, index: Inode) -> &mut Self::Output {
		let idx = index as usize;
		let u8_ref = &mut self.bytes[idx * self.inode_size];
		let ptr = u8_ref as *mut u8 as *mut InodeData;
		unsafe { &mut *ptr }
	}
}

type Inode = u32;
type Block = u32;
type Group = u32;

use Ext2Err::*;
/// Error related to the ext2 filesystem, or things it requires
#[derive(Copy, Clone, Debug)]
pub enum Ext2Err {
	/// An error related to file I/O
	IO(IOError),
	/// An error related to a name
	Name,
	/// Out of Inodes
	NoInodes,
	/// Out of Blocks
	NoBlocks,
	/// Not absolute file path
	NotAbsolute,
	/// The specified file path was not found
	FileNotFound,
}

impl From<IOError> for Ext2Err {
	fn from(e: IOError) -> Self {
		IO(e)
	}
}

impl From<FromUtf8Error> for Ext2Err {
	fn from(e: FromUtf8Error) -> Self {
		Name
	}
}

/// Set up an ext2 disk, and all necessary data structures to go along with it.
pub fn setup() -> Result<(), Ext2Err> {
	let partition = partitions::get_ext2_partition().unwrap();
	unsafe {
		DEVICE = Some(Mutex::new(partition));
	}

	let disk = Ext2::read_from_disk()?;
	unsafe { EXT = Some(Mutex::new(disk)) }

	// let mut alice_reader = FileReader::new(11);
	// let mut data = Vec::new();
	// alice_reader.read_to_end(&mut data)?;
	// let string = String::from_utf8(data);
	// serial_println!("{}", string.unwrap());

	// let mut root_reader = FileReader::new(2);
	// let directory = Directory::read(&mut root_reader)?;
	// for item in directory.entries {
	// 	serial_println!("{:?}", item);
	// }
	let inode = path_to_inode("/alice.txt")?;
	serial_println!("{}", inode);
	let mut alice_reader = FileReader::new(inode);
	alice_reader.write(b"Hello world, ")?;
	alice_reader.flush()?;

	Ok(())
}

/// Write back all unsaved changes (to the super block, block group descriptors, etc) to the disk
pub fn cleanup() -> Result<(), Ext2Err> {
	get_ext!().lock().write_to_disk()
}

// BORDER WALL
// ------------------------------------------------------------------------------------------

// fn find_free_inode(super_block: &SuperBlock, device: &Mutex<dyn BlockDevice>) -> Option<u32> {
// 	for group in 0..super_block.num_blockgroups() {
// 		let mut block_reader = BlockReader::new(
// 			super_block.block_group_start_block(group) as usize,
// 			super_block.sectors_per_block(),
// 			0,
// 			device,
// 		);

// 		block_reader.seek(SeekFrom::Current(
// 			(group as usize * size_of::<BlockGroupDescriptor>()) as isize,
// 		));

// 		let mut block_group_desc: BlockGroupDescriptor = unsafe { block_reader.read_type().unwrap() };
// 		let free = block_group_desc.unallocated_inodes;
// 		if free == 0 {
// 			break;
// 		} else {
// 			// Remove the inode from free
// 			block_group_desc.unallocated_inodes -= 1;
// 			block_reader.seek(SeekFrom::Current(-(size_of::<BlockGroupDescriptor>() as isize)));
// 			block_reader.write_type(&block_group_desc);
// 			block_reader.flush();

// 			let mut new_superblock: SuperBlock = *super_block;
// 			new_superblock.unallocated_inodes -= 1;
// 			let mut sector_reader = BlockReader::new(2, 1, 0, device);
// 			sector_reader.write_type(&new_superblock);
// 			sector_reader.flush();

// 			block_reader.move_to_block(block_group_desc.inode_bitmap_addr);
// 			for i in 0..super_block.block_size() / 8 {
// 				let int: u8 = unsafe { block_reader.read_type().unwrap() };
// 				if int == u8::MAX {
// 					continue;
// 				}

// 				let mut val = int;
// 				for j in 0..8 {
// 					if val % 2 == 0 {

// fn find_path_inode(path: &str, super_block: &SuperBlock, device: &Mutex<dyn BlockDevice>) -> Option<u32> {
// 	let mut split = path.split("/");

// 	// Get rid of empty string before the first /
// 	split.next();
// 	// Root Inode
// 	let mut inode = 2;
// 	for name in split {
// 		let file_reader = FileReader::new(inode, super_block, device);
// 		let mut directory_iter = DirectoryIter { reader: file_reader };
// 		let result = directory_iter.find(|(entry, entryname)| name == entryname);
// 		match result {
// 			None => return None,
// 			Some((entry, _)) => {
// 				inode = entry.inode;
// 			}
// 		}
// 	}

// 	Some(inode)
// }

// fn get_inode_data<'a>(
// 	inode: u32,
// 	super_block: &'a SuperBlock,
// 	block_device: &'a Mutex<dyn BlockDevice>,
// ) -> (BlockGroupDescriptor, InodeData, BlockReader<'a>) {
// 	let group = super_block.inode_blockgroup(inode);
// 	let mut block_reader = BlockReader::new(
// 		super_block.block_group_start_block(group) as usize,
// 		super_block.sectors_per_block(),
// 		0,
// 		block_device,
// 	);

// 	block_reader.seek(SeekFrom::Current(
// 		(group as usize * size_of::<BlockGroupDescriptor>()) as isize,
// 	));

// 	let block_group_desc: BlockGroupDescriptor = unsafe { block_reader.read_type().unwrap() };
// 	serial_println!("{:?}", block_group_desc);
// 	serial_println!("");
// 	block_reader.move_to_block(block_group_desc.inode_table_addr);
// 	block_reader.seek(SeekFrom::Current(
// 		(super_block.inode_index_in_blockgroup(inode) as usize * super_block.inode_size as usize) as isize,
// 	));
// 	let inode_data: InodeData = unsafe { block_reader.read_type().unwrap() };
// 	(block_group_desc, inode_data, block_reader)
// }

// fn remove_inode(inode: u32, super_block: &SuperBlock, block_device: &Mutex<dyn BlockDevice>) {
// 	let (block_group_desc, inode_data, mut block_reader) = get_inode_data(inode, super_block, block_device);
// 	serial_println!("{:?}", inode_data);
// 	let block_iter: InodeBlockIter<true> = InodeBlockIter::new(inode_data, block_reader.clone());

// 	// Mark inode free in inode bitmap
// 	{
// 		let inode_bitmap = block_reader.read_block(block_group_desc.inode_bitmap_addr);
// 		let inode_index_in_blockgroup = super_block.inode_index_in_blockgroup(inode);
// 		let index_in_slice = inode_index_in_blockgroup as usize / 8;
// 		// zero the corresponding bit
// 		inode_bitmap[index_in_slice] &= !((1 << (inode_index_in_blockgroup % 8)) as u8);
// 		block_reader.write_current_block();
// 	}

// 	let group = super_block.inode_blockgroup(inode);
// 	let start_of_block_group = super_block.block_group_start_block(group);
// 	let mut freed_blocks = 0;
// 	// Mark blocks free in block bitmap
// 	{
// 		let block_bitmap = block_reader.read_block(block_group_desc.block_bitmap_addr);
// 		// serial_println!("Start of block group {}", start_of_block_group);
// 		for block in block_iter {
// 			// TODO figure out if and why +1 is correct
// 			let block_index = block + 1 - start_of_block_group;

// 			assert!(
// 				block_index < super_block.blocks_in_blockgroup,
// 				"Trying to free block in another group"
// 			);

// 			let index_in_slice = block_index as usize / 8;

// 			assert!(
// 				(block_bitmap[index_in_slice] >> (block_index % 8)) % 2 == 1,
// 				"Freeing free block? at block {}: {}",
// 				freed_blocks,
// 				block
// 			);

// 			// zero the corresponding bit
// 			block_bitmap[index_in_slice] &= !((1 << (block_index % 8)) as u8);
// 			freed_blocks += 1;
// 		}
// 		block_reader.write_current_block();
// 		serial_println!("Blocks freed: {}", freed_blocks);
// 	}

// 	// fix block group descriptor (free inode count, free block count)
// 	block_reader.move_to_block(start_of_block_group);
// 	block_reader.seek(SeekFrom::Current(
// 		(group as usize * size_of::<BlockGroupDescriptor>()) as isize,
// 	));
// 	let mut new_descriptor = block_group_desc;
// 	new_descriptor.unallocated_blocks += freed_blocks as u16;
// 	new_descriptor.unallocated_inodes += 1;
// 	block_reader.write_type(&new_descriptor);
// 	block_reader.flush();

// 	// fix superblock (free inode count, free block count)
// 	let mut new_superblock: SuperBlock = *super_block;
// 	new_superblock.unallocated_inodes += 1;
// 	new_superblock.unallocated_blocks += freed_blocks;
// 	let mut sector_reader = BlockReader::new(2, 1, 0, block_device);
// 	sector_reader.write_type(&new_superblock);
// 	sector_reader.flush();
// }

// struct InodeBlockIter<'a, const WITHPARENTS: bool> {
// 	blocks: [VecDeque<u32>; 4],
// 	inode_data: InodeData,
// 	reader: BlockReader<'a>,
// }

// impl<'a, const WITHPARENTS: bool> InodeBlockIter<'a, WITHPARENTS> {
// 	fn get_next_block_of(&mut self, level: usize) -> Option<u32> {
// 		match self.blocks[level].pop_front() {
// 			Some(0) => None,
// 			Some(block) => Some(block),
// 			None => {
// 				let parent = self.get_next_block_of(level + 1);
// 				match parent {
// 					Some(block) => {
// 						let slice = self.reader.read_block(block);
// 						let sub_blocks;
// 						unsafe {
// 							sub_blocks = slice_from_raw_parts(slice.as_ptr() as *const u32, slice.len() / 4)
// 								.as_ref()
// 								.unwrap();
// 							self.blocks[level].extend(sub_blocks);
// 						}
// 						if WITHPARENTS {
// 							parent
// 						} else {
// 							let new_block = self.blocks[level].pop_front().unwrap();
// 							assert_ne!(new_block, 0);
// 							Some(new_block)
// 						}
// 					}
// 					None => None,
// 				}
// 			}
// 		}
// 	}

// 	fn new(inode_data: InodeData, reader: BlockReader<'a>) -> Self {
// 		let direct = inode_data.direct_block_pointers;
// 		let indirect = inode_data.singly_indirect_pointer;
// 		let double_indirect = inode_data.doubly_indirect_pointer;
// 		let triple_indirect = inode_data.triply_indirect_pointer;
// 		let direct_blocks = VecDeque::from(direct);
// 		let singly_indirect_blocks = if indirect == 0 {
// 			VecDeque::new()
// 		} else {
// 			VecDeque::from([indirect])
// 		};
// 		let doubly_indirect_blocks = if indirect == 0 {
// 			VecDeque::new()
// 		} else {
// 			VecDeque::from([double_indirect])
// 		};
// 		let tripy_indirect_blocks = if indirect == 0 {
// 			VecDeque::new()
// 		} else {
// 			VecDeque::from([triple_indirect])
// 		};
// 		Self {
// 			inode_data,
// 			blocks: [
// 				direct_blocks,
// 				singly_indirect_blocks,
// 				doubly_indirect_blocks,
// 				tripy_indirect_blocks,
// 			],
// 			reader,
// 		}
// 	}
// }

// impl<const WITHPARENTS: bool> Iterator for InodeBlockIter<'_, WITHPARENTS> {
// 	type Item = u32;
// 	fn next(&mut self) -> Option<Self::Item> {
// 		self.get_next_block_of(0)
// 	}
// }

// struct DirectoryIter<'a> {
// 	reader: FileReader<'a>,
// }

// impl<'a> Iterator for DirectoryIter<'a> {
// 	type Item = (DirectoryEntry, String);

// 	fn next(&mut self) -> Option<Self::Item> {
// 		let result = unsafe { self.reader.read_type::<DirectoryEntry>() };
// 		if let Ok(entry) = result {
// 			let len = entry.total_entry_size as usize - size_of::<DirectoryEntry>();
// 			let mut name = vec![0u8; len];
// 			if self.reader.read(&mut name).is_err() {
// 				return None;
// 			}
// 			name.truncate(entry.name_length_low as usize);
// 			Some((entry, String::from_utf8(name).unwrap()))
// 		} else {
// 			None
// 		}
// 	}
// }
