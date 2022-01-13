use crate::mem::paging;
use bootloader::boot_info::MemoryRegions;

use super::pci;

/// Setup AHCI
pub fn setup(regions: &MemoryRegions) {
	let functions = pci::recursive_scan();
	let res = functions
		.iter()
		.find(|func| pci::get_class_code(**func) == 0x01 && pci::get_subclass_code(**func) == 0x06);
	match res {
		Some(function) => {
			serial_println!("Found AHCI device: {:?}", function);
			let abar = pci::get_bars(*function)[5];
			serial_println!("ABAR: {:?}", abar);
			let address;
			match abar {
				pci::Bar::MemorySpace {
					prefetchable,
					base_address,
				} => {
					address = base_address;
				}
				_ => {
					unreachable!()
				}
			}
			let virt_addr = paging::phys_to_virt(address);
			unsafe {
				let mut base_ptr = virt_addr.as_u64();
				let version_major_ones = *((base_ptr + 0x12) as *const u8);
				let version_major_tens = *((base_ptr + 0x13) as *const u8);
				let version_minor_ones = *((base_ptr + 0x10) as *const u8);
				let version_minor_tens = *((base_ptr + 0x11) as *const u8);
				serial_println!(
					"AHCI version: {}{}.{}{}",
					version_major_tens,
					version_major_ones,
					version_minor_tens,
					version_minor_tens
				);
			}
		}
		None => {
			serial_println!("No AHCI device, cannot access storage!");
		}
	}
}
