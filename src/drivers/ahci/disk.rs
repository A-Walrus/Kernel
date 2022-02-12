use super::{fis::RecievedFis, CommandList, CommandTable, Port, ReadWrite};
use crate::{drivers::pci::Function, mem::heap::UBox};

trait BlockDevice {
	/// Will always return 512
	fn sector_size(&self) -> usize;

	/// The number of sectors in this device
	fn num_sectors(&self) -> usize;
}

// pub struct Partition<T: BlockDevice> {
//     start_sector:usize,
//     length:usize,
//     disk: TODO something with T
// }

pub struct AtaDisk {
	num_sectors: usize,
	port: &'static mut Port,
	// TODO possibly eventually refactor to use these instead of PhysAddrs
	// recieved_fis: &'static mut RecievedFis,
	// command_list: &'static mut CommandList,
	// command_tables: [&'static mut CommandTable; 32],
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
	fn sector_size(&self) -> usize {
		return 512;
	}

	fn num_sectors(&self) -> usize {
		self.num_sectors
	}
}
