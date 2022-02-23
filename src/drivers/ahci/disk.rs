use crate::{
	mem::heap::{UBox, UBuffer},
	util::io::{IOError, Read},
};
use alloc::boxed::Box;
use core::{
	cmp::min,
	mem::{size_of, zeroed},
	ptr::{slice_from_raw_parts, slice_from_raw_parts_mut},
	slice,
};
use spin::Mutex;

use super::{Port, Sector, SECTOR_SIZE};

/// A struct that can browse and read sectors, with a similair interface to std::io
pub struct BlockReader<'a> {
	offset: usize,
	block: usize,
	sectors_per_block: usize,
	buffer: UBuffer,
	block_device: &'a Mutex<dyn BlockDevice>,
}

impl<'a> BlockReader<'a> {
	/// Create a new SectorReador at a certain sector, and offset, on a given block device (through
	/// Mutex for safety)
	pub fn new(
		block: usize,
		sectors_per_block: usize,
		offset: usize,
		block_device: &'a Mutex<dyn BlockDevice>,
	) -> Self {
		Self {
			offset,
			block,
			sectors_per_block,
			block_device,
			buffer: UBuffer::new(sectors_per_block * SECTOR_SIZE),
		}
	}

	/// Read the current block into the buffer
	fn read_current_block(&mut self) {
		// serial_println!("Reading block: {}, of size: {}", self.block, self.sectors_per_block);
		let slice;
		unsafe {
			// slice = slice_from_raw_parts_mut(self.buffer.ptr as *mut Sector, self.sectors_per_block)
			// 	.as_mut()
			// 	.unwrap();
			slice = slice_from_raw_parts_mut(self.buffer.slice as *mut Sector, self.sectors_per_block)
				.as_mut()
				.unwrap();
		}
		self.block_device
			.lock()
			.read_sectors(self.block * self.sectors_per_block, slice);
	}

	/// Move the "cursor" forward some offset of bytes, possibly crossing sectors and block
	/// boundaries
	pub fn seek_forward(&mut self, offset: usize) {
		let new_offset = (self.offset + offset) % (self.sectors_per_block * SECTOR_SIZE);
		let block_offset = (self.offset + offset) / (self.sectors_per_block * SECTOR_SIZE);
		if (self.offset == 0 || block_offset != 0) && new_offset != 0 {
			self.read_current_block();
		}
		self.block += block_offset;
		self.offset = new_offset;
	}

	/// Read the block into the buffer and return a slice to it
	pub fn read_block(&mut self, block: u32) -> &[u8] {
		self.move_to_block(block as usize);
		self.read_current_block();
		self.slice()
	}

	/// Move to block
	pub fn move_to_block(&mut self, block: usize) {
		self.block = block;
		self.offset = 0;
	}

	/// Get the slice of the buffer
	pub fn slice(&self) -> &[u8] {
		unsafe { self.buffer.slice.as_ref().unwrap() }
	}

	/// Get the mut slice of the buffer
	pub fn mut_slice(&mut self) -> &mut [u8] {
		unsafe { self.buffer.slice.as_mut().unwrap() }
	}
	// /// Write to the current location from the buffer
	// pub fn write(&mut self, buffer: &[u8]) {
	// 	unimplemented!()
	// }
}

impl<'a> Read for BlockReader<'a> {
	/// Fill the buffer with bytes red from the current location
	fn read(&mut self, mut buffer: &mut [u8]) -> Result<usize, IOError> {
		let original_length = buffer.len();
		while buffer.len() > 0 {
			if self.offset == 0 {
				self.read_current_block();
				if buffer.len() >= self.sectors_per_block * SECTOR_SIZE {
					self.block += 1;
				}
			}
			let len_available = SECTOR_SIZE * self.sectors_per_block - self.offset;
			let len_to_take = min(len_available, buffer.len());
			unsafe {
				// serial_println!("{} {} {}", self.offset, len_to_take, len_available);
				buffer[..len_to_take]
					.copy_from_slice(&self.buffer.slice.as_mut().unwrap()[self.offset..(self.offset + len_to_take)]);
			}
			buffer = &mut buffer[len_to_take..];
			// self.offset += len_to_take;
			self.offset = if len_to_take == len_available {
				0
			} else {
				self.offset + len_to_take
			};
		}
		Ok(original_length) // TODO failable read
	}
}
/// Trait representing a block device.
pub trait BlockDevice {
	/// Will always return 512
	fn sector_size(&self) -> usize {
		return SECTOR_SIZE;
	}

	/// The number of sectors in this device
	fn num_sectors(&self) -> usize;

	/// Read sector at LBA
	fn read_sector(&mut self, lba: usize, buffer: &mut Sector) {
		self.read_sectors(lba, slice::from_mut(buffer));
	}

	/// Write sector at LBA
	fn write_sector(&mut self, lba: usize, buffer: &Sector) {
		unsafe {
			let temp = &mut *(slice::from_ref(buffer) as *const [Sector] as *mut [Sector]);
			self.write_sectors(lba, temp);
		}
	}

	/// Read sectors at LBA
	fn read_sectors(&mut self, lba: usize, buffer: &mut [Sector]);

	/// Write sectors at LBA
	fn write_sectors(&mut self, lba: usize, buffer: &[Sector]);
}

/// Struct represnting a partition on some block device
pub struct Partition {
	start_sector: usize,
	length: usize,
	disk: Box<dyn BlockDevice>,
}

impl BlockDevice for Partition {
	fn num_sectors(&self) -> usize {
		self.length
	}

	fn read_sectors(&mut self, lba: usize, buffer: &mut [Sector]) {
		self.disk.read_sectors(lba + self.start_sector, buffer);
	}

	fn write_sectors(&mut self, lba: usize, buffer: &[Sector]) {
		self.disk.write_sectors(lba + self.start_sector, buffer);
	}
}

impl Partition {
	/// Create a new partiton on a given disk. Taking the disk. (TODO change the partition type to
	/// be able to have multiple partitions on one disk
	pub fn new(start_sector: usize, length: usize, disk: Box<dyn BlockDevice>) -> Self {
		assert!(start_sector + length < disk.num_sectors());
		Self {
			start_sector,
			length,
			disk,
		}
	}
}

/// Structure represnting an ATA disk. Can be used through trait
pub struct AtaDisk {
	num_sectors: usize,
	port: &'static mut Port,
}

impl AtaDisk {
	/// Create a new ATA disk around a port. Calling this multiple times for a given port will
	/// cause UB
	pub unsafe fn new(port: &'static mut Port) -> Self {
		let disk_data;
		port.rebase();
		disk_data = port.ata_identify().expect("Failed to Identify disk");
		// let mut buffer = UBox::new([[5; 512]; 8]);
		// port.ata_dma(0, &mut *buffer, ReadWrite::Read).expect("Failed to read");
		// buffer[0][0] = 5;
		// port.ata_dma(0, &mut *buffer, ReadWrite::Write).expect("Failed to read");
		// port.ata_dma(0, &mut *buffer, ReadWrite::Read).expect("Failed to read");
		Self {
			port,
			num_sectors: disk_data.sector_count,
		}
	}
}
use super::ReadWrite::*;

impl BlockDevice for AtaDisk {
	fn num_sectors(&self) -> usize {
		self.num_sectors
	}

	fn read_sectors(&mut self, lba: usize, buffer: &mut [Sector]) {
		assert!(
			lba + buffer.len() < self.num_sectors,
			"Trying to read outside of sector"
		);
		unsafe {
			self.port
				.ata_dma(lba as u64, buffer, Read)
				.expect("Failed to read sector");
		}
	}

	fn write_sectors(&mut self, lba: usize, buffer: &[Sector]) {
		assert!(
			lba + buffer.len() < self.num_sectors,
			"Trying to write outside of sector"
		);
		unsafe {
			self.port
				.ata_dma(lba as u64, &mut *(buffer as *const [Sector] as *mut [Sector]), Write)
				.expect("Failed to write sector");
		}
	}
}
