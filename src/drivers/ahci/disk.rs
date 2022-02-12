use super::Port;

pub trait BlockDevice {
	/// Will always return 512
	fn sector_size(&self) -> usize {
		return 512;
	}

	/// The number of sectors in this device
	fn num_sectors(&self) -> usize;

	// fn read();

	// fn write();
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

impl BlockDevice for AtaDisk {
	fn num_sectors(&self) -> usize {
		self.num_sectors
	}
}
