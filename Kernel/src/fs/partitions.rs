use crate::{
	drivers::{ahci, ahci::disk::Partition},
	mem::heap::UBox,
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
pub fn get_ext2_partition() -> Option<Partition> {
	let disks;
	unsafe {
		disks = ahci::get_disks().expect("Failed to get disk");
	}
	let mut buffer = UBox::new([0; 512]);
	for mut disk in disks {
		// Buffer to store gpt header
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
				const LINUX_FILE_SYSTEM: [u8; 16] = [
					0xAF, 0x3D, 0xC6, 0xF, 0x83, 0x84, 0x72, 0x47, 0x8E, 0x79, 0x3D, 0x69, 0xD8, 0x47, 0x7D, 0xE4,
				];
				if gpt_partition.type_guid == LINUX_FILE_SYSTEM {
					println!(
						"FOUND LINUX FILE SYSTEM! Sectors : {} -> {}",
						gpt_partition.first_lba, gpt_partition.last_lba
					);
					return Some(Partition::new(
						gpt_partition.first_lba,
						gpt_partition.last_lba - gpt_partition.first_lba,
						disk,
					));
				}
			}
		}
	}
	None
}
