use crate::{
	drivers::ahci,
	mem::{self, heap::UBox},
};

#[repr(C)]
struct GptHeader {
	signature: [u8; 8],
	revision: [u8; 4],
	_junk: [u8; 0x50 - 0xC],
	num_entries: u32,
}

#[repr(C)]
#[derive(Debug)]
struct GptPartition {
	type_guid: [u8; 16],
	unique_guid: [u8; 16],
	first_lba: usize,
	last_lba: usize,
}

impl GptHeader {
	fn check_signature(&self) -> Result<(), ()> {
		if self.signature == [0x45, 0x46, 0x49, 0x20, 0x50, 0x41, 0x52, 0x54] {
			Ok(())
		} else {
			Err(())
		}
	}
}

/// Find the ext2 partition
pub fn get_ext2_partition() {
	let mut disk;
	unsafe {
		disk = ahci::get_disk().expect("Failed to get disk");
	}

	// Buffer to store gpt header
	let mut buffer = UBox::new([0; 512]);
	let buffer_ref = &mut *buffer;
	disk.read_sector(1, buffer_ref);
	unsafe {
		let gpt_header = &*(buffer_ref as *mut _ as *const GptHeader);
		gpt_header.check_signature().expect("Invalid gpt signature {}");
	}
	for partition in 0..32 {
		disk.read_sector(2 + partition, buffer_ref);
		unsafe {
			let gpt_partition = &*(buffer_ref as *mut _ as *const GptPartition);
			serial_println!("{:?}", gpt_partition);
		}
	}
}
