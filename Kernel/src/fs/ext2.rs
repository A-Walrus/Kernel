use super::partitions;
use crate::{
	cpu::syscalls::OpenFlags,
	drivers::ahci::disk::{BlockReader, Partition},
	util::io::*,
};
use alloc::{
	str,
	string::{FromUtf8Error, String, ToString},
	vec::Vec,
};
use core::{
	cmp::min,
	mem::size_of,
	ops::{Index, IndexMut},
	ptr::slice_from_raw_parts_mut,
	slice,
};
use spin::Mutex;

static mut DEVICE: Option<Mutex<Partition>> = None;

static mut EXT: Option<Mutex<Ext2>> = None;

const ROOT_INODE: Inode = 2;

const SEPARATOR: &'static str = "/"; // I think I could have chosen anything as the separator, but '/' was chosen like UNIX
const SELF: &'static str = "."; // In ext, the . represents an entry in a directory pointing to itself
const PARENT: &'static str = ".."; // In ext, the .. represents an entry in a directory pointing to its parent

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
	fn check_signature(&self) -> Result<(), Ext2Err> {
		if self.signature == 61267 {
			Ok(())
		} else {
			Err(InvalidSignature)
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
		(group * self.blocks_in_blockgroup) + start
	}

	fn num_blockgroups(&self) -> u32 {
		(self.blocks + (self.blocks_in_blockgroup / 2)) / self.blocks_in_blockgroup
	}

	fn block_blockgroup(&self, block: Block) -> u32 {
		(block - self.block_group_start_block(0)) / self.blocks_in_blockgroup
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

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Type {
	Fifo = 5,
	CharacterDevice = 3,
	Directory = 2,
	BlockDevice = 4,
	RegularFile = 1,
	SymbolicLink = 7,
	UnixSocket = 6,
	Other = 0,
}

#[derive(Debug, Copy, Clone)]
struct TypeAndPermissions(u16);

impl TypeAndPermissions {
	fn inode_type(&self) -> Type {
		match self.0 >> 12 {
			0x1 => Type::Fifo,
			0x2 => Type::CharacterDevice,
			0x4 => Type::Directory,
			0x6 => Type::BlockDevice,
			0x8 => Type::RegularFile,
			0xA => Type::SymbolicLink,
			0xC => Type::UnixSocket,
			_ => Type::Other,
		}
	}

	fn new(inode_type: Type, permissions: u16) -> Self {
		let val = match inode_type {
			Type::Fifo => 0x1,
			Type::CharacterDevice => 0x2,
			Type::Directory => 0x4,
			Type::BlockDevice => 0x6,
			Type::RegularFile => 0x8,
			Type::SymbolicLink => 0xA,
			Type::UnixSocket => 0xC,
			Type::Other => 0, // This shouldn't happen
		};
		TypeAndPermissions(val << 12 | (permissions & 0xFFF))
	}
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
/// Literal structure found on disk, the inode
struct InodeData {
	type_and_permissions: TypeAndPermissions,
	user_id: u16,
	size_lower: u32,
	last_access_time: u32,
	creation_time: u32,
	last_modification_time: u32,
	deletion_time: u32,
	group_id: u16,
	hard_link_count: u16,
	sectors_in_use: u32, // Disk sectors (512b), Not ext2 Blocks (1K+)
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
	type_indicator: Type,
}

/// Some ext fields store their log minus 10, this undos that operation.
fn undo_log_minus_10(num: u32) -> usize {
	1 << (num + 10)
}

// End of literal structs

/// Abstract custom struct representing an easy to work with directory
#[derive(Debug)]
/// A directory
pub struct Directory {
	/// Vec of entries in directory
	pub entries: Vec<Entry>,
}

impl Directory {
	/// Get directory from path
	pub fn from_path(path: &str) -> Result<Directory, Ext2Err> {
		let mut file = File::from_path(path, OpenFlags::empty())?;
		Directory::read(&mut file)
	}

	fn read(reader: &mut File) -> Result<Self, Ext2Err> {
		if reader.inode_data.type_and_permissions.inode_type() != Type::Directory {
			return Err(NotADir);
		}

		const ENTRY_SIZE: usize = size_of::<DirectoryEntry>();

		let mut entries = Vec::new();
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes)?;
		// serial_println!("{:?}", bytes);
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

	fn empty() -> Self {
		Self { entries: Vec::new() }
	}

	fn write(&mut self, writer: &mut File) -> Result<usize, Ext2Err> {
		let mut written = 0;

		let block_size = writer.reader.slice().len();
		for entries in &mut self.entries.windows(2) {
			let entry = &entries[0];
			let pos = writer.position;
			let offset = pos % block_size;
			let total_entry_size: u16 = if offset + entries[0].min_size() + entries[1].min_size() > block_size {
				(block_size - offset) as u16
			} else {
				entry.min_size() as u16
			};

			let mut dir_entry = entry.entry;
			written += total_entry_size;
			dir_entry.total_entry_size = total_entry_size;

			writer.write_type(&dir_entry)?;

			writer.write(entry.name.as_bytes())?;
			writer.write(&[0u8])?; // String should be null terminated

			writer.seek(SeekFrom::Current(
				total_entry_size as isize - entry.actual_size() as isize,
			))?;
		}
		let pos = writer.position;
		let offset = pos % block_size;
		let last = self.entries.last().unwrap();
		let mut dir_entry = last.entry;

		let total_entry_size = (block_size - offset) as u16;
		written += total_entry_size;
		dir_entry.total_entry_size = total_entry_size;

		writer.write_type(&dir_entry)?;

		writer.write(last.name.as_bytes())?;
		writer.write(&[0u8])?; // String should be null terminated

		writer.inode_data.size_lower = written as u32;

		Ok(written as usize)
	}
}

// impl IntoIterator for Directory {
// type Item = Entry;
// type IntoIter = alloc::vec::IntoIter<Entry>;
// fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
// self.entries.into_iter()
// }
// }

/// Struct representing an entry in an abstract directory
#[derive(Clone, Debug)]
pub struct Entry {
	/// Entry data
	entry: DirectoryEntry,
	/// Entry name
	pub name: String,
}

impl Entry {
	fn min_size(&self) -> usize {
		// (1 + ((size_of::<DirectoryEntry>() + self.name.len() - 1) / 4)) * 4
		let actual_size = size_of::<DirectoryEntry>() + self.name.len();
		(actual_size + 3) & !3
	}

	fn actual_size(&self) -> usize {
		size_of::<DirectoryEntry>() + self.name.len() + 1
	}
}

struct Ext2 {
	super_block: SuperBlock,
	block_groups: Vec<BlockGroup>,
}

impl Ext2 {
	fn get_inode_data(&self, inode: Inode) -> &InodeData {
		serial_println!("Getting inode data {}", inode);
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
		serial_println!("{:?}", super_block);
		super_block.check_signature()?;

		let mut block_reader = BlockReader::new(0, super_block.sectors_per_block(), 0, block_device);

		let mut block_groups = Vec::new();
		for group_index in 0..super_block.num_blockgroups() {
			serial_println!("Reading group: {}", group_index);
			block_reader.move_to_block(super_block.block_group_start_block(0))?;

			block_reader.seek(SeekFrom::Current(
				(group_index as usize * size_of::<BlockGroupDescriptor>()) as isize,
			))?;

			let descriptor: BlockGroupDescriptor = unsafe { block_reader.read_type()? };
			serial_println!("Descriptor: {:?}", descriptor);

			let table_size = super_block.inodes_in_blockgroup as usize * super_block.inode_size as usize;
			let mut blocks = vec![0; table_size];
			block_reader.move_to_block(descriptor.inode_table_addr)?;
			block_reader.read(&mut blocks)?;

			let inode_table = InodeTable {
				inode_size: super_block.inode_size as usize,
				bytes: blocks,
			};

			let bytes = Vec::from(block_reader.read_block(descriptor.inode_bitmap_addr)?);
			let inode_bitmap = ExtBitMap { bytes };

			let bytes = Vec::from(block_reader.read_block(descriptor.block_bitmap_addr)?);
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
		sector_reader.flush()?;

		let mut block_reader = BlockReader::new(0, self.super_block.sectors_per_block(), 0, block_device);

		serial_println!("Write to disk");

		for (i, group) in self.block_groups.iter().enumerate() {
			serial_println!("writing group {}", i);
			block_reader.move_to_block(group.first_block)?;
			for group in &self.block_groups {
				block_reader.write_type(&group.descriptor)?;
			}

			block_reader.move_to_block(group.descriptor.inode_bitmap_addr)?;
			block_reader.write(&group.inode_bitmap.bytes)?;

			block_reader.move_to_block(group.descriptor.block_bitmap_addr)?;
			block_reader.write(&group.block_bitmap.bytes)?;

			block_reader.move_to_block(group.descriptor.inode_table_addr)?;
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
		let group = self.super_block.block_blockgroup(block);
		let group = &mut self.block_groups[group as usize];
		group.free_block(block)?;
		self.super_block.unallocated_blocks += 1;
		serial_println!(
			"Freed block {}, free block count is now: {}",
			block,
			self.super_block.unallocated_blocks
		);
		Ok(())
	}

	fn free_inode(&mut self, inode: Inode) -> Result<(), Ext2Err> {
		let group = self.super_block.inode_blockgroup(inode);
		let group = &mut self.block_groups[group as usize];
		group.free_inode(inode)?;
		self.super_block.unallocated_inodes += 1;
		// serial_println!(
		// 	"Freed inode {}, free inode count is now: {}",
		// 	inode,
		// 	self.super_block.unallocated_inodes
		// );
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
			Ok(self.first_inode + (position as u32))
		}
	}

	/// Try to get a free block, mark it as used
	fn get_free_block(&mut self) -> Result<Block, Ext2Err> {
		if self.descriptor.unallocated_blocks == 0 {
			Err(NoBlocks)
		} else {
			self.descriptor.unallocated_blocks -= 1;
			let index = self.block_bitmap.get_free().unwrap();
			let block = index as u32 + self.first_block - 1;
			// Ok(self.first_block + (index as u32))
			Ok(block)
		}
	}

	fn free_inode(&mut self, inode: Inode) -> Result<(), Ext2Err> {
		self.descriptor.unallocated_inodes += 1;
		let index = (inode - self.first_inode) as usize;
		self.inode_bitmap.set(index, Free);
		Ok(())
	}

	fn free_block(&mut self, block: Block) -> Result<(), Ext2Err> {
		self.descriptor.unallocated_blocks += 1;
		let index = (block - self.first_block + 1) as usize;
		self.block_bitmap.get(index as usize);
		self.block_bitmap.set(index, Free);
		Ok(())
	}
}

struct ExtBitMap {
	bytes: Vec<u8>,
}

impl ExtBitMap {
	fn set(&mut self, place: usize, value: Bit) {
		// serial_println!("setting {} to {:?}", place, value);
		let index = place / 8;
		let offset = place % 8;
		self.bytes[index] = match value {
			Free => self.bytes[index] & !(1 << offset),
			Used => self.bytes[index] | (1 << offset),
		}
	}

	fn get(&self, place: usize) -> Bit {
		let index = place / 8;
		let offset = place % 8;
		if self.bytes[index] & (1 << offset) == 0 {
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

					return Some(((i * 8) + j) as usize);
				} else {
					val = val >> 1;
				}
			}
		}
		None
	}
}

/// Add a regular file at a given path
pub fn add_regular_file(path: &str) -> Result<Inode, Ext2Err> {
	// TODO check that path is okay before doing anything
	//  - Absolute
	//  - Parent exists,
	//  - It doesn't exist

	let inode_data = InodeData {
		type_and_permissions: TypeAndPermissions::new(Type::RegularFile, 0b000110110110),
		user_id: 0,
		size_lower: 0,
		last_access_time: 0,
		creation_time: 0,
		last_modification_time: 0,
		deletion_time: 0,
		group_id: 0,
		hard_link_count: 0, // will be 1 once linked
		sectors_in_use: 0,
		flags: 0,
		os_specific_val1: 0,
		direct_block_pointers: [0; 12],
		singly_indirect_pointer: 0,
		doubly_indirect_pointer: 0,
		triply_indirect_pointer: 0,
		generation_number: 0,
		file_acl: 0,
		size_upper_or_directory_acl: 0,
		fragment_block_address: 0,
		os_specific_val2: [0; 12],
	};
	let inode = add_inode(inode_data)?;
	serial_println!("Adding new file at inode: {}", inode);
	link(path, inode)?;
	Ok(inode)
}

fn add_inode(data: InodeData) -> Result<Inode, Ext2Err> {
	let mut ext = get_ext!().lock();
	let free_inode: Inode = ext.get_free_inode()?;
	let data_mut = ext.get_inode_data_mut(free_inode);
	*data_mut = data;

	Ok(free_inode)
}

/// Create a (hard) link to an Inode. returns the parent inode
pub fn link(path: &str, inode: Inode) -> Result<Inode, Ext2Err> {
	let index = path.rfind(SEPARATOR).unwrap();
	let folder_path = &path[..index + 1];
	let file_name = &path[index + 1..];

	let dir_inode = path_to_inode(folder_path)?;

	let mut parent_reader = File::new(dir_inode)?;
	let mut directory = Directory::read(&mut parent_reader)?;

	if directory.entries.iter().find(|entry| entry.name == file_name).is_some() {
		// File already exists
		Err(FileAlreadyExists)
	} else {
		let mut ext = get_ext!().lock();
		let inode_data = ext.get_inode_data_mut(inode);
		inode_data.hard_link_count += 1;

		directory.entries.push(Entry {
			name: file_name.to_string(),
			entry: DirectoryEntry {
				inode,
				total_entry_size: 0, // Doesn't matter, will get overwritten,
				name_length_low: file_name.len() as u8,
				type_indicator: inode_data.type_and_permissions.inode_type(),
			},
		});

		parent_reader.rewind()?;
		directory.write(&mut parent_reader)?;

		Ok(dir_inode)
	}
}

/// Create a directory
pub fn mkdir(path: &str) -> Result<Inode, Ext2Err> {
	// TODO check that path is okay before doing anything
	serial_println!("mkdir {}", path);
	let inode_data = InodeData {
		type_and_permissions: TypeAndPermissions::new(Type::Directory, 0b000111101101),
		user_id: 0,
		size_lower: 0,
		last_access_time: 0,
		creation_time: 0,
		last_modification_time: 0,
		deletion_time: 0,
		group_id: 0,
		hard_link_count: 1, // will be 2 once linked (self, and from parent)
		sectors_in_use: 0,
		flags: 0,
		os_specific_val1: 0,
		direct_block_pointers: [0; 12],
		singly_indirect_pointer: 0,
		doubly_indirect_pointer: 0,
		triply_indirect_pointer: 0,
		generation_number: 0,
		file_acl: 0,
		size_upper_or_directory_acl: 0,
		fragment_block_address: 0,
		os_specific_val2: [0; 12],
	};

	let inode = add_inode(inode_data)?;
	serial_println!("Added inode");

	let parent_inode = link(path, inode)?;
	serial_println!("Linked");

	let mut directory = Directory::empty();
	directory.entries.push(Entry {
		name: SELF.to_string(),
		entry: DirectoryEntry {
			inode,
			total_entry_size: 0, // Doesn't matter, will get overwritten,
			name_length_low: SELF.len() as u8,
			type_indicator: Type::Directory,
		},
	});
	directory.entries.push(Entry {
		name: PARENT.to_string(),
		entry: DirectoryEntry {
			inode: parent_inode,
			total_entry_size: 0, // Doesn't matter, will get overwritten,
			name_length_low: PARENT.len() as u8,
			type_indicator: Type::Directory,
		},
	});

	let mut writer = File::new(inode)?;
	directory.write(&mut writer)?;

	{
		let mut ext = get_ext!().lock();
		// Update directory count
		let block_group = ext.super_block.inode_blockgroup(inode) as usize;
		let group = &mut ext.block_groups[block_group];
		group.descriptor.dirs_in_group += 1;

		// Update parent link count
		ext.get_inode_data_mut(parent_inode).hard_link_count += 1;
	}
	Ok(inode)
}

/// Remov an empty (only . and ..) directory
pub fn rmdir(path: &str) -> Result<(), Ext2Err> {
	let inode = path_to_inode(path)?;
	serial_println!("Rmdir Inode: {} ", inode);

	let parent_inode;
	{
		let mut reader = File::new(inode)?;
		let directory = Directory::read(&mut reader)?;
		if directory.entries.len() > 2 {
			return Err(DirNotEmpty);
		}

		parent_inode = directory
			.entries
			.iter()
			.find_map(|entry| {
				if entry.name == PARENT {
					Some(entry.entry.inode)
				} else {
					None
				}
			})
			.map_or(Err(NoParentDir), |a| Ok(a))?;
	}
	serial_println!("Parent inode: {} ", parent_inode);
	// Update directory count
	{
		let mut ext = get_ext!().lock();
		let block_group = ext.super_block.inode_blockgroup(inode) as usize;
		let group = &mut ext.block_groups[block_group];
		group.descriptor.dirs_in_group -= 1;
	}

	unlink_inode(inode)?;
	unlink_inode(parent_inode)?;
	unlink(path)?;
	Ok(())
}

/// Remove a link to an inode
fn unlink_inode(inode: Inode) -> Result<(), Ext2Err> {
	let sectors_per_block = get_ext!().lock().super_block.sectors_per_block();
	let inode_data = *get_ext!().lock().get_inode_data(inode);

	let mut ext = get_ext!().lock();
	let device = get_device!();
	if inode_data.hard_link_count == 1 {
		// Get rid of inode
		ext.free_inode(inode)?;
		let mut b_reader = BlockReader::new(0, sectors_per_block, 0, device);
		let blocks = get_inode_blocks(inode_data, &mut b_reader, true)?;
		for block in blocks {
			ext.free_block(block)?;
		}
		unsafe {
			let inode_data_ptr = (ext.get_inode_data_mut(inode)) as *mut InodeData;
			inode_data_ptr.write_bytes(0, 1);
		}
	} else {
		// Decrease link count
		ext.get_inode_data_mut(inode).hard_link_count -= 1;
	}
	Ok(())
}

/// Unlink a file, also called removing. If there are multiple hard links to the file, the
/// other links will continue to be able to access it
pub fn unlink(path: &str) -> Result<(), Ext2Err> {
	let inode = path_to_inode(path)?;
	unlink_inode(inode)?;

	let index = path.rfind(SEPARATOR).unwrap();
	let folder_path = &path[..index + 1];
	let file_name = &path[index + 1..];
	serial_println!("file name: {} ", file_name);
	serial_println!("folder path: {} ", folder_path);

	let dir_inode = path_to_inode(folder_path)?;
	serial_println!("folder inode: {} ", dir_inode);

	let mut parent_reader = File::new(dir_inode)?;
	let mut directory = Directory::read(&mut parent_reader)?;

	let prev_len = directory.entries.len();
	directory.entries.retain(|entry| entry.name != file_name);
	let new_len = directory.entries.len();
	assert!(new_len + 1 == prev_len);

	parent_reader.rewind()?;
	directory.write(&mut parent_reader)?;

	Ok(())
}

fn path_to_inode(mut path: &str) -> Result<Inode, Ext2Err> {
	if *path == *SEPARATOR {
		return Ok(ROOT_INODE);
	}

	if !path.starts_with(SEPARATOR) {
		return Err(NotAbsolute);
	}

	if path.ends_with(SEPARATOR) {
		// Directory style syntax
		path = &path[..path.len() - 1];
	}

	let mut split = path.split(SEPARATOR);

	// Get rid of empty string before the first /
	split.next();

	// Root Inode
	let mut inode: Inode = ROOT_INODE;

	for name in split {
		let mut file_reader = File::new(inode)?;
		let directory = Directory::read(&mut file_reader)?;
		// serial_println!("Searching for: {}", name);
		// serial_println!("Directory : {:#?}", directory);
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

fn get_inode_blocks(inode: InodeData, b_reader: &mut BlockReader, with_parents: bool) -> Result<Vec<Block>, Ext2Err> {
	let mut blocks = Vec::new();

	let result = inode.direct_block_pointers.iter().position(|val| *val == 0);
	match result {
		Some(index) => {
			blocks.extend_from_slice(&inode.direct_block_pointers[..index]);
		}
		None => {
			blocks.extend_from_slice(&inode.direct_block_pointers);
			if inode.singly_indirect_pointer == 0 {
				return Ok(blocks);
			}
			get_indirect_blocks(&mut blocks, b_reader, inode.singly_indirect_pointer, 1, with_parents)?;
			get_indirect_blocks(&mut blocks, b_reader, inode.doubly_indirect_pointer, 2, with_parents)?;
			get_indirect_blocks(&mut blocks, b_reader, inode.triply_indirect_pointer, 3, with_parents)?;
		}
	}

	Ok(blocks)
}

fn get_sub_blocks(b_reader: &mut BlockReader, block: Block) -> Result<&'static mut [Block], IOError> {
	let slice = b_reader.read_block(block)?;

	unsafe {
		let slice = slice_from_raw_parts_mut(slice.as_ptr() as *mut Block, slice.len() / 4)
			.as_mut()
			.unwrap();
		Ok(slice)
	}
}

fn get_indirect_blocks(
	blocks: &mut Vec<Block>,
	b_reader: &mut BlockReader,
	block: Block,
	indirectness: usize,
	with_parents: bool,
) -> Result<(), Ext2Err> {
	if block == 0 {
		return Ok(());
	} else {
		let mut sub_blocks = &*get_sub_blocks(b_reader, block)?;

		let zero_index = sub_blocks.iter().position(|val| *val == 0);
		if let Some(index) = zero_index {
			sub_blocks = &sub_blocks[..index]
		}
		if indirectness > 1 {
			if with_parents {
				blocks.extend_from_slice(sub_blocks);
			}
			for block in sub_blocks {
				get_indirect_blocks(blocks, b_reader, *block, indirectness, with_parents)?;
			}
		} else {
			if with_parents {
				blocks.push(block);
			}
			blocks.extend_from_slice(sub_blocks);
		}
	}
	Ok(())
}

/// File handle
#[derive(Debug)]
pub struct File {
	inode: Inode,
	inode_data: InodeData,
	reader: BlockReader,
	position: usize,
	blocks: Vec<Block>,
}

impl File {
	/// get file handle from path
	pub fn from_path(path: &str, flags: OpenFlags) -> Result<Self, Ext2Err> {
		let inode = path_to_inode(path);
		match inode {
			Ok(inode) => File::new(inode),
			Err(FileNotFound) => {
				if flags.contains(OpenFlags::CREATE) {
					let new_inode = add_regular_file(path)?;
					File::new(new_inode)
				} else {
					Err(FileNotFound)
				}
			}
			Err(e) => Err(e),
		}
	}

	fn new(inode: u32) -> Result<Self, Ext2Err> {
		serial_println!("Opening inode: {}", inode);
		let ext = get_ext!();
		let device = get_device!();
		let inode_data = *ext.lock().get_inode_data(inode);
		let super_block = ext.lock().super_block;

		let mut block_reader = BlockReader::new(0, super_block.sectors_per_block(), 0, device);
		let blocks = get_inode_blocks(inode_data, &mut block_reader, false)?;
		Ok(Self {
			inode,
			inode_data,
			reader: block_reader,
			position: 0,
			blocks,
		})
	}
}
impl Read for File {
	fn read(&mut self, mut buf: &mut [u8]) -> Result<usize, IOError> {
		let to_read = min(buf.len(), self.inode_data.size_lower as usize - self.position);
		let mut left_to_read = to_read;

		let block_size = self.reader.slice().len();
		let mut block;
		while left_to_read > 0 {
			let offset_in_block = self.position % block_size;
			let to_read_from_block = min(left_to_read, block_size - offset_in_block);
			let next_block = self.blocks[self.position / block_size];
			block = self.reader.read_block(next_block)?;
			buf[..to_read_from_block].copy_from_slice(&block[offset_in_block..offset_in_block + to_read_from_block]);
			buf = &mut buf[to_read_from_block..];
			left_to_read -= to_read_from_block;
			self.position += to_read_from_block;
		}
		Ok(to_read)
	}
}

impl Seek for File {
	fn seek(&mut self, pos: SeekFrom) -> Result<usize, IOError> {
		match pos {
			SeekFrom::Start(offset) => {
				self.position = offset;
			}
			SeekFrom::Current(offset) => {
				self.position = (self.position as isize + offset) as usize;
			}
			SeekFrom::End(_offset) => unimplemented!(),
		}
		// Seeking after the end of a file is allowed, behaviour is to leave whatever data was
		// there already.
		if self.position as u32 > self.inode_data.size_lower {
			self.inode_data.size_lower = self.position as u32
		}
		Ok(self.position)
	}
}

impl Write for File {
	fn write(&mut self, mut buf: &[u8]) -> Result<usize, IOError> {
		let old_block_count = self.blocks.len();

		let to_write = buf.len();
		let mut left_to_write = to_write;

		let block_size = self.reader.slice().len();
		let mut block;
		while left_to_write > 0 {
			let offset_in_block = self.position % block_size;
			let to_write_to_block = min(left_to_write, block_size - offset_in_block);

			let next_block = match self.blocks.get(self.position / block_size) {
				Some(block) => *block,
				None => {
					let free_block = get_ext!().lock().get_free_block().map_err(|_| IOError::Other)?;
					self.blocks.push(free_block);

					free_block
				}
			};

			block = self.reader.read_block(next_block)?;
			block[offset_in_block..offset_in_block + to_write_to_block].copy_from_slice(&buf[..to_write_to_block]);
			buf = &buf[to_write_to_block..];
			left_to_write -= to_write_to_block;
			self.position += to_write_to_block;
		}

		let added_blocks = &self.blocks[old_block_count..];

		if self.position as u32 > self.inode_data.size_lower {
			self.inode_data.size_lower = self.position as u32
		}

		// Update added blocks
		if !added_blocks.is_empty() {
			let mut new_blocks = added_blocks.len();

			if self.blocks.len() <= 12 {
				// Only direct blocks
				self.inode_data.direct_block_pointers[..self.blocks.len()].copy_from_slice(&self.blocks);
			} else {
				let block_size = self.reader.slice().len();
				let blocks_per_block = block_size / size_of::<Block>();

				let old0 = old_block_count;
				let new0 = self.blocks.len();
				// serial_println!("0: {}	->	{}", old0, new0);

				let old1 = next_tier_blocks(old0, 0, blocks_per_block);
				let new1 = next_tier_blocks(new0, 0, blocks_per_block);
				// serial_println!("1: {}	->	{}", old1, new1);

				let old2 = next_tier_blocks(old1, 1, blocks_per_block);
				let new2 = next_tier_blocks(new1, 1, blocks_per_block);
				// serial_println!("2: {}	->	{}", old2, new2);

				let old3 = next_tier_blocks(old2, 2, blocks_per_block);
				let new3 = next_tier_blocks(new2, 2, blocks_per_block);
				// serial_println!("3: {}	->	{}", old3, new3);

				let mut reader = self.reader.clone();

				let a = fill_block_pointers(
					&mut reader,
					old3,
					new3,
					slice::from_mut(&mut self.inode_data.triply_indirect_pointer),
					&[],
				)?;
				let b = fill_block_pointers(
					&mut reader,
					old2,
					new2,
					slice::from_mut(&mut self.inode_data.doubly_indirect_pointer),
					&a,
				)?;
				let c = fill_block_pointers(
					&mut reader,
					old1,
					new1,
					slice::from_mut(&mut self.inode_data.singly_indirect_pointer),
					&b,
				)?;
				// do_thing(&mut reader, old0, new0, &mut self.inode_data.direct_block_pointers, &c)?;
				{
					let old = old0;
					let in_inode = &mut self.inode_data.direct_block_pointers;
					let mut parents: &[Block] = &c;
					let mut slice: &[Block] = added_blocks;
					if old < in_inode.len() {
						let len = min(in_inode.len() - old, slice.len());
						in_inode[old..old + len].copy_from_slice(&slice[..len]);
						slice = &slice[len..]
					}

					while !slice.is_empty() {
						let sub_blocks = get_sub_blocks(&mut reader, parents[0])?;
						parents = &parents[1..];
						let first_zero = sub_blocks
							.iter()
							.position(|a| *a == 0)
							.expect("No free blocks in parent");

						let sub_slice = &mut sub_blocks[first_zero..];
						let len = min(slice.len(), sub_slice.len());
						sub_slice[..len].copy_from_slice(&slice[..len]);
						slice = &slice[len..];
					}
				}
				new_blocks += new1 + new2 + new3 - old1 - old2 - old3;
			}

			// Update used sector count
			let added_sector_count = new_blocks * self.reader.sectors_per_block();
			self.inode_data.sectors_in_use += added_sector_count as u32;
		}

		Ok(to_write)
	}

	fn flush(&mut self) -> Result<(), IOError> {
		self.reader.flush()
	}
}

fn fill_block_pointers(
	mut reader: &mut BlockReader,
	old: usize,
	new: usize,
	in_inode: &mut [Block],
	mut parents: &[Block],
) -> Result<Vec<Block>, IOError> {
	let mut vec = Vec::new();
	let mut first: Option<Block> = None;

	if old > 0 && old <= in_inode.len() {
		// First is the last non zero
		let first_zero = in_inode.iter().position(|a| *a == 0);
		first = match first_zero {
			None => Some(in_inode[in_inode.len() - 1]),
			Some(index) => Some(in_inode[index - 1]),
		}
	}

	for _ in 0..new - old {
		let free_block = get_ext!().lock().get_free_block().map_err(|_| IOError::Other)?;
		vec.push(free_block);
	}

	let mut slice: &[Block] = &vec;
	if old < in_inode.len() {
		let len = min(in_inode.len() - old, slice.len());
		in_inode[old..old + len].copy_from_slice(&slice[..len]);
		slice = &slice[len..]
	}

	while !slice.is_empty() {
		let sub_blocks = get_sub_blocks(&mut reader, parents[0])?;
		parents = &parents[1..];
		let first_zero = sub_blocks
			.iter()
			.position(|a| *a == 0)
			.expect("No free blocks in parent");
		if first.is_none() {
			first = Some(sub_blocks[first_zero - 1]);
		}

		let sub_slice = &mut sub_blocks[first_zero..];
		let len = min(slice.len(), sub_slice.len());
		sub_slice[..len].copy_from_slice(&slice[..len]);
		slice = &slice[len..];
	}

	// If old == 0 vec is only new blocks, if it isn't than vec is the last block + the new blocks
	// The last block is either the last (nonzero) block in in_inode, or the last (nonzero) block in
	// the sub blocks of the first parent.

	match first {
		None => {
			assert_eq!(old, 0);
		}
		Some(block) => {
			vec.insert(0, block);
		}
	}

	if old == 0 {
		assert_eq!(vec.len(), new);
	} else {
		assert_eq!(vec.len(), new - old + 1);
	}
	Ok(vec)
}

/// Get number of blocks required in next tier
fn next_tier_blocks(count: usize, tier: usize, bpb: usize) -> usize {
	let in_inode = if tier == 0 { 12 } else { 1 };
	if count <= in_inode {
		0
	} else {
		(count - in_inode + bpb - 1) / bpb
	}
}

impl Drop for File {
	fn drop(&mut self) {
		let ext = get_ext!();
		*ext.lock().get_inode_data_mut(self.inode) = self.inode_data;
	}
}

use Bit::*;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
	/// An error related to file/disk I/O
	IO(IOError),
	/// An error related to Utf8
	Utf8Error,
	/// Out of Inodes
	NoInodes,
	/// Out of Blocks
	NoBlocks,
	/// Not absolute file path
	NotAbsolute,
	/// The specified file path was not found
	FileNotFound,
	/// The file you are trying to create already exists
	FileAlreadyExists,
	/// Trying to do a directory operation on a file that is not a directory
	NotADir,
	/// Trying to do a file operation on a file that is not a regular file
	NotAFile,
	/// Trying to do an operation that only works on empty directories on a non empty one
	DirNotEmpty,
	/// No parent dir. All directories should have a parent (..)
	NoParentDir,
	/// Ext2 signature is invalid
	InvalidSignature,
	/// Handle doesn't exist
	NoHandle,
	/// No more entries in this directory
	EndOfDir,
}

impl From<IOError> for Ext2Err {
	fn from(e: IOError) -> Self {
		IO(e)
	}
}

impl From<FromUtf8Error> for Ext2Err {
	fn from(_e: FromUtf8Error) -> Self {
		Utf8Error
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

	Ok(())
}

/// Do some things to the file system
pub fn test() {
	let inode = mkdir("/new_directory").expect("Failed to create directory");
	println!("Directory inode: {}", inode);

	let inode = add_regular_file("/new_directory/new_file.txt").expect("Failed to add file");
	println!("File indoe: {}", inode);
	let mut writer = File::new(inode).expect("failed ot open file");
	writer.write(b"Hello world!\n").expect("Failed to write");
	writer.write(b"My name is Guy\n").expect("Failed to write");
	for _ in 0..10 {
		writer.write(TEST_DATA).expect("failed to write");
	}

	// add_regular_file("/other_file.txt").expect("Failed to add file");
	// rmdir("/bar").expect("Failed to delete dir");

	// let inode = path_to_inode("/alice.txt").expect("failed ot open inode");
	// let mut writer = File::new(inode).expect("failed ot open file");
	// writer.write(b"hello eran").expect("Failed to write");
}

/// Read entire file into Vec
pub fn read_file(path: &str) -> Result<Vec<u8>, Ext2Err> {
	let inode = path_to_inode(path)?;
	let mut reader = File::new(inode)?;
	let mut data = Vec::new();
	reader.read_to_end(&mut data)?;
	Ok(data)
}

/// Write back all unsaved changes (to the super block, block group descriptors, etc) to the disk
pub fn cleanup() -> Result<(), Ext2Err> {
	get_ext!().lock().write_to_disk()
}

const TEST_DATA:&[u8; 4116] = b"
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vestibulum vehicula interdum justo ut tincidunt. Mauris accumsan at ipsum quis convallis. Ut elementum laoreet venenatis. Fusce orci neque, convallis sit amet magna sit amet, scelerisque pellentesque lectus. Ut rhoncus arcu ut commodo vehicula. Morbi sed eros non nunc ornare bibendum. Phasellus ut rhoncus ipsum. Etiam posuere dolor at suscipit ullamcorper.

Suspendisse ac pharetra quam. Donec et nisi tellus. Aenean et purus risus. Sed porttitor nibh id felis bibendum, eu malesuada neque vestibulum. Donec metus turpis, hendrerit in magna ac, aliquet ornare massa. Sed odio urna, dignissim eu dui sit amet, fringilla blandit lorem. In porta fringilla pellentesque. Donec consectetur diam lectus, vitae sollicitudin nisl elementum quis. Pellentesque ut dictum elit, eget efficitur lorem. Nam eu tempor erat, semper lobortis metus. Sed cursus erat ullamcorper tellus porta, ut accumsan enim placerat. Vivamus lacinia eget est eget ullamcorper. Nam ligula arcu, feugiat sit amet tellus sit amet, dignissim rutrum mi. Pellentesque eget felis ac nunc pulvinar aliquam.

Mauris non mauris porttitor, pellentesque neque sed, mattis orci. Integer rhoncus risus vel molestie ullamcorper. Phasellus cursus vehicula elit, eu pharetra mi pellentesque eget. Proin purus lacus, tincidunt ut cursus ac, ultrices sit amet ante. Sed semper pharetra arcu, vel vestibulum velit viverra et. Integer id dictum mi. Morbi sed condimentum risus. Nullam bibendum, ex id facilisis semper, nunc libero ultricies lorem, ac vulputate mi sem nec ipsum. Mauris tincidunt augue nisl, id posuere lorem tincidunt ac.

Phasellus vestibulum ipsum vel tortor eleifend, ac placerat quam facilisis. Morbi dapibus, lorem id lobortis dignissim, lorem eros fermentum sem, et tincidunt dui orci in ante. Integer lobortis, leo ac interdum vehicula, orci nisl ullamcorper libero, eget scelerisque risus justo ac tellus. Nulla venenatis id turpis non placerat. Aliquam at massa eu lacus vehicula dapibus. Donec pellentesque arcu ut tellus rutrum eleifend. Suspendisse imperdiet sed nulla in ullamcorper. Phasellus dictum dapibus sem sit amet blandit. Fusce bibendum tellus rhoncus, lacinia dolor ut, iaculis magna. Vestibulum consectetur quis augue et blandit.

Nam iaculis, odio a efficitur lacinia, risus ipsum pulvinar massa, in efficitur ex elit aliquam libero. Praesent aliquet congue felis id fringilla. Proin at nunc aliquet, interdum metus tempus, tincidunt justo. Nam pretium venenatis ex, vel vestibulum turpis eleifend quis. Donec eu lobortis est. Integer lectus ante, finibus in odio in, auctor placerat augue. Vestibulum a tincidunt massa, vitae volutpat lectus.

Quisque et viverra dui, quis varius nulla. Phasellus iaculis quam fermentum rhoncus facilisis. Nunc tempus gravida justo quis egestas. Nam euismod ac lectus vitae lacinia. Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia curae; Suspendisse potenti. Integer a viverra elit. Ut eget lacinia lorem.

Nullam gravida consectetur lacus vel vehicula. Phasellus ac magna dui. Quisque eleifend vitae leo ut blandit. Nunc arcu turpis, rhoncus in ex a, tincidunt mollis mauris. Sed quis posuere arcu. Curabitur ultrices sapien hendrerit eros commodo blandit. Donec aliquam lacus quis quam facilisis dignissim.

Nulla dapibus magna purus, a laoreet sem porta sed. Sed ultricies mauris ac arcu cursus, ac cursus neque blandit. Vestibulum faucibus suscipit hendrerit. Integer id risus varius, molestie metus quis, pulvinar leo. In sit amet erat quis sapien tincidunt dignissim nec et sapien. Nullam aliquet elementum orci, et accumsan metus laoreet eget. Sed accumsan risus et erat fringilla, non ultrices urna consectetur.

Praesent pretium purus quis nibh interdum, vel ornare diam rutrum. Pellentesque vel mattis mauris. Aliquam vestibulum sagittis est, non viverra nibh pharetra vel. Donec hendrerit magna quis diam dictum, et malesuada lacus blandit. Ut elit velit, ornare vel enim in, imperdiet sollicitudin nibh. Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere volutpat. 	
";
