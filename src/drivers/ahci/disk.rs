//use crate::drivers::pci::Function;

//trait BlockDevice {
//	/// Will always return 512
//	pub fn sector_size(&self) -> usize;

//	pub fn num_sectors(&self) -> usize;
//}

//struct AtaDisk {
//	num_sectors: usize,
//}

//impl BlockDevice for AtaDisk {
//	fn new(function: Function, slot: u32) -> Self {}

//	fn sector_size(&self) -> usize {
//		return 512;
//	}

//	fn num_sectors(&self) -> usize {
//		self.num_sectors
//	}
//}
////
