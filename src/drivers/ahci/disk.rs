use core::slice;

use super::{Port, Sector};

pub trait BlockDevice {
	/// Will always return 512
	fn sector_size(&self) -> usize {
		return 512;
	}

	/// The number of sectors in this device
	fn num_sectors(&self) -> usize;

	/// Read sector at LBA
	fn read_sector(&mut self, lba: usize, buffer: &mut Sector);

	/// Write sector at LBA
	fn write_sector(&mut self, lba: usize, buffer: &Sector);
}

pub struct Partition<T: BlockDevice> {
	start_sector: usize,
	length: usize,
	disk: T,
}

impl<T: BlockDevice> BlockDevice for Partition<T> {
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

impl<T: BlockDevice> Partition<T> {
	pub fn new(start_sector: usize, length: usize, disk: T) -> Self {
		assert!(start_sector + length < disk.num_sectors());
		Self {
			start_sector,
			length,
			disk,
		}
	}
}

pub struct AtaDisk {
	num_sectors: usize,
	port: &'static mut Port,
}

impl AtaDisk {
	pub fn new(port: &'static mut Port) -> Self {
		let disk_data;
		unsafe {
			port.rebase();
			disk_data = port.ata_identify().expect("Failed to Identify disk");
			// let mut buffer = UBox::new([[5; 512]; 8]);
			// port.ata_dma(0, &mut *buffer, ReadWrite::Read).expect("Failed to read");
			// buffer[0][0] = 5;
			// port.ata_dma(0, &mut *buffer, ReadWrite::Write).expect("Failed to read");
			// port.ata_dma(0, &mut *buffer, ReadWrite::Read).expect("Failed to read");
		}
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
