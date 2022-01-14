use core::fmt::{Debug, Display};

use crate::mem::paging;
use bootloader::boot_info::MemoryRegions;

use super::pci;

#[repr(C)]
struct AHCIVersion {
	minor: [u8; 2],
	major: [u8; 2],
}

impl Display for AHCIVersion {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(
			f,
			"{}{}.{}{}",
			self.major[1], self.major[0], self.minor[1], self.minor[0]
		)
	}
}

impl Debug for AHCIVersion {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "{}", self)
	}
}

#[derive(Debug)]
#[repr(C)]
struct HBAPort {
	command_list_base: u32,       // base address 1KiB aligned
	command_list_base_upper: u32, // base address upper 32 bits
	fis_base_address: u32,
	fis_base_address_upper: u32,
	interrupt_status: u32,
	interrupt_enable: u32,
	command_and_status: u32,
	_reserved0: u32,
	task_file_data: u32,
	signature: u32,
	sata_status: u32,
	sata_control: u32,
	sata_error: u32,
	sata_active: u32,
	command_issue: u32,
	sata_notification: u32,
	fis_based_switch_control: u32,
	_reserved2: [u8; 11],
	vendor_specific: [u32; 4],
}

#[derive(Debug)]
#[repr(C)]
struct HBAMemory {
	capabilities: u32,
	global_host_control: u32,
	interrupt_status: u32,
	port_implemented: u32,
	version: AHCIVersion,
	ccc_control: u32, // Command comletion coalescing control
	ccc_ports: u32,   // Command comletion coalescing ports
	enclosure_management_location: u32,
	enclosure_management_control: u32,
	capabilities_extended: u32,
	bios_os_handoff: u32,
	_reserved: [u8; 0xA0 - 0x2C],
	vendor_specific: [u8; 0x100 - 0xA0],
	ports: [HBAPort; 32],
}

/// Setup AHCI
pub fn setup() {
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
					prefetchable: _,
					base_address,
				} => {
					address = base_address;
				}
				_ => {
					unreachable!()
				}
			}
			let virt_addr = paging::phys_to_virt(address);
			let hba_memory: &mut HBAMemory;
			unsafe {
				hba_memory = &mut *(virt_addr.as_mut_ptr());
			}
			serial_println!("{:#?}", hba_memory);
		}
		None => {
			serial_println!("No AHCI device, cannot access storage!");
		}
	}
}
