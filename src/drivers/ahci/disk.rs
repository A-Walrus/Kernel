use crate::{
	mem::heap::UBuffer,
	util::io::{IOError, Read, Seek, SeekFrom, Write},
};
use alloc::boxed::Box;
use core::{cmp::min, ptr::slice_from_raw_parts_mut, slice};
use spin::Mutex;

use super::{Port, Sector, SECTOR_SIZE};

/// A struct that can browse and read sectors, with a similair interface to std::io
#[derive(Clone)]
pub struct BlockReader<'a> {
	offset: usize,
	block: usize,
	sectors_per_block: usize,
	buffer: UBuffer,
	block_in_buffer: Option<usize>,
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
			block_in_buffer: None,
			buffer: UBuffer::new(sectors_per_block * SECTOR_SIZE),
		}
	}

	/// Get the current block
	fn get_current_block(&mut self) {
		match self.block_in_buffer {
			None => self.read_current_block(),
			Some(block) => {
				if block != self.block {
					self.read_current_block()
				}
			}
		}
	}

	/// Read the current block into the buffer
	fn read_current_block(&mut self) {
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
		self.block_in_buffer = Some(self.block);
	}

	/// Write the current block to the disk
	pub fn write_current_block(&mut self) {
		// serial_println!("writing to block {}", self.block);
		let slice;
		unsafe {
			slice = slice_from_raw_parts_mut(self.buffer.slice as *mut Sector, self.sectors_per_block)
				.as_mut()
				.unwrap();
		}
		self.block_device
			.lock()
			.write_sectors(self.block * self.sectors_per_block, slice);
	}

	/// Read the block into the buffer and return a slice to it
	pub fn read_block(&mut self, block: u32) -> Result<&mut [u8], IOError> {
		self.move_to_block(block)?;
		self.get_current_block();
		Ok(self.mut_slice())
	}

	/// Move to block
	pub fn move_to_block(&mut self, block: u32) -> Result<(), IOError> {
		self.flush()?;
		self.block = block as usize;
		self.offset = 0;
		Ok(())
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

impl<'a> Seek for BlockReader<'a> {
	fn seek(&mut self, pos: SeekFrom) -> Result<usize, IOError> {
		self.flush()?;
		match pos {
			SeekFrom::Start(offset) => {
				self.block = offset / (self.sectors_per_block * SECTOR_SIZE);
				self.offset = offset % (self.sectors_per_block * SECTOR_SIZE);
				Ok(offset) // TODO make failable
			}
			SeekFrom::Current(offset) => {
				let new_offset = (self.offset as isize + offset)
					.rem_euclid((self.sectors_per_block * SECTOR_SIZE) as isize) as usize;
				let block_offset = (self.offset as isize + offset) / (self.sectors_per_block * SECTOR_SIZE) as isize;
				self.block = (self.block as isize + block_offset) as usize;
				self.offset = new_offset;
				Ok(self.block * self.sectors_per_block * SECTOR_SIZE + self.offset)
			}
			SeekFrom::End(_offset) => {
				unimplemented!()
			}
		}
	}
}

impl<'a> Read for BlockReader<'a> {
	/// Fill the buffer with bytes red from the current location
	fn read(&mut self, mut buffer: &mut [u8]) -> Result<usize, IOError> {
		let original_length = buffer.len();
		while buffer.len() > 0 {
			self.get_current_block();

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
				self.block += 1;
				0
			} else {
				self.offset + len_to_take
			};
		}
		Ok(original_length) // TODO failable read
	}
}

impl<'a> Write for BlockReader<'a> {
	fn write(&mut self, mut buffer: &[u8]) -> Result<usize, IOError> {
		let original_length = buffer.len();
		while buffer.len() > 0 {
			self.get_current_block();
			let len_available = SECTOR_SIZE * self.sectors_per_block - self.offset;
			let len_to_take = min(len_available, buffer.len());
			unsafe {
				// serial_println!("{} {} {}", self.offset, len_to_take, len_available);
				// buffer[..len_to_take]
				// 	.copy_from_slice(&self.buffer.slice.as_mut().unwrap()[self.offset..(self.offset + len_to_take)]);
				self.buffer.slice.as_mut().unwrap()[self.offset..(self.offset + len_to_take)]
					.copy_from_slice(&buffer[..len_to_take]);
			}
			buffer = &buffer[len_to_take..];
			// self.offset += len_to_take;
			self.offset = if len_to_take == len_available {
				self.write_current_block();
				self.block += 1;
				0
			} else {
				self.offset + len_to_take
			};
		}
		Ok(original_length) // TODO failable write
	}

	fn flush(&mut self) -> Result<(), IOError> {
		match self.block_in_buffer {
			None => Ok(()),
			Some(block) => {
				if block == self.block {
					self.write_current_block();
				}
				Ok(())
			}
		}
	}
}

impl<'a> Drop for BlockReader<'a> {
	fn drop(&mut self) {
		self.flush().expect("Failed to flush on BlockReader Drop");
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
