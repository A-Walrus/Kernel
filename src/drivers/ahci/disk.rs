use crate::mem::heap::UBox;
use alloc::boxed::Box;
use core::{
	mem::{size_of, zeroed},
	ptr::{slice_from_raw_parts, slice_from_raw_parts_mut},
	slice,
};
use spin::Mutex;

use super::{Port, Sector, SECTOR_SIZE};

/// A struct that can browse and read sectors, with a similair interface to std::io
pub struct SectorReader<'a> {
	offset: usize,
	sector: usize,
	buffer: UBox<Sector>,
	block_device: &'a Mutex<dyn BlockDevice>,
}

impl<'a> SectorReader<'a> {
	/// Create a new SectorReador at a certain sector, and offset, on a given block device (through
	/// Mutex for safety)
	pub fn new(sector: usize, offset: usize, block_device: &'a Mutex<dyn BlockDevice>) -> Self {
		let mut me = Self {
			offset,
			sector,
			block_device,
			buffer: UBox::new([0; SECTOR_SIZE]),
		};
		me.read_current_sector();
		me
	}

	fn read_current_sector(&mut self) {
		self.block_device.lock().read_sector(self.sector, &mut *self.buffer);
	}

	/// Fill the buffer with bytes red from the current location
	pub fn read(&mut self, mut buffer: &mut [u8]) {
		let mut len_wanted = buffer.len();
		let mut len_available = SECTOR_SIZE - self.offset;

		while len_wanted >= len_available {
			buffer[..len_available].copy_from_slice(&self.buffer[self.offset..SECTOR_SIZE]);
			buffer = &mut buffer[len_available..];

			self.sector += 1;
			self.read_current_sector();

			self.offset = 0;
			len_available = SECTOR_SIZE;
			len_wanted = buffer.len();
		}
		buffer[..len_wanted].copy_from_slice(&self.buffer[self.offset..self.offset + len_wanted]);
	}

	/// Read data into a struct.
	/// # Safety
	/// - Must make sure that the data in that part of the disk is valid for that type, otherwise
	/// UB
	#[inline(always)]
	pub unsafe fn read_type<T>(&mut self) -> T {
		let mut val: T = zeroed();
		let slice = &mut *(slice_from_raw_parts_mut(&mut val as *mut T as *mut u8, size_of::<T>()));
		self.read(slice);
		val
	}
	/// Write to the current location from the buffer
	pub fn write(&mut self, buffer: &[u8]) {
		unimplemented!()
	}

	/// Move reader to given sector and offset
	pub fn move_to(&mut self, sector: usize, offset: usize) {
		self.sector = sector;
		self.offset = offset;
		self.read_current_sector();
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
	fn read_sector(&mut self, lba: usize, buffer: &mut Sector);

	/// Write sector at LBA
	fn write_sector(&mut self, lba: usize, buffer: &Sector);
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

	fn read_sector(&mut self, lba: usize, buffer: &mut Sector) {
		self.disk.read_sector(lba + self.start_sector, buffer);
	}

	fn write_sector(&mut self, lba: usize, buffer: &Sector) {
		self.disk.write_sector(lba + self.start_sector, buffer);
	}
}

impl Partition {
	/// Create a new partiton on a given disk. Taking the disk. (TODO change the partition type to
	/// be able to hav emultiple partitions on one disk
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

	fn read_sector(&mut self, lba: usize, buffer: &mut Sector) {
		assert!(lba < self.num_sectors, "Trying to read outside of sector");
		unsafe {
			self.port
				.ata_dma(lba as u64, slice::from_mut(buffer), Read)
				.expect("Failed to read sector");
		}
	}

	fn write_sector(&mut self, lba: usize, buffer: &Sector) {
		assert!(lba < self.num_sectors, "Trying to write outside of sector");
		unsafe {
			self.port
				.ata_dma(
					lba as u64,
					&mut *(slice::from_ref(buffer) as *const [Sector] as *mut [Sector]),
					Write,
				)
				.expect("Failed to write sector");
		}
	}
}
