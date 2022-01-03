use x86_64::instructions::port::{Port, PortGeneric, ReadOnlyAccess, ReadWriteAccess, WriteOnlyAccess};

const CONFIG_ADDRESS: PortGeneric<u32, WriteOnlyAccess> = PortGeneric::new(0xCF8);
const CONFIG_DATA: PortGeneric<u32, ReadOnlyAccess> = PortGeneric::new(0xCFC);

use crate::{serial_print, serial_println};

/// test pci
pub fn testing() {
	brute_force_scan();
}

// Config address format
// |--------+------------+------------+------------+--------------+-----------------|
// | Bit 31 | Bits 30-24 | Bits 23-16 | Bits 15-11 | Bits 10-8    | Bits 7-0        |
// |--------+------------+------------+------------+--------------+-----------------|
// | Enable | Reserved   | Bus Num    | Device Num | Function Num | Register Offset |
// |--------+------------+------------+------------+--------------+-----------------|

// Read a word at a certain bus, slot, func, offset
fn pci_config_read(bus: u8, slot: u8, func: u8, register: u8) -> u32 {
	let lbus = bus as u32;
	let lslot = slot as u32;
	let lfunc = func as u32;
	let lregister = register as u32;

	let address: u32 = lbus << 16 | lslot << 11 | lfunc << 8 | lregister << 2 | 0x80000000;

	unsafe {
		CONFIG_ADDRESS.write(address);
	}

	unsafe { CONFIG_DATA.read() }
}

fn brute_force_scan() {
	for bus in 0..=255 {
		for device in 0..32 {
			check_device(bus, device);
		}
	}
}

fn get_vendor_id(bus: u8, device: u8, func: u8) -> u16 {
	let reg = pci_config_read(bus, device, func, 0);
	reg as u16
}

fn get_header_type(bus: u8, device: u8, func: u8) -> u8 {
	let reg = pci_config_read(bus, device, func, 3);
	(reg >> 16) as u8
}

fn get_class_code(bus: u8, device: u8, func: u8) -> u8 {
	let reg = pci_config_read(bus, device, func, 2);
	(reg >> 24) as u8
}

fn get_subclass_code(bus: u8, device: u8, func: u8) -> u8 {
	let reg = pci_config_read(bus, device, func, 2);
	(reg >> 16) as u8
}

fn check_device(bus: u8, device: u8) {
	if get_vendor_id(bus, device, 0) == 0xFFFF {
		// Device doesn't exist
	} else {
		// Device exists
		check_function(bus, device, 0);
		let header_type = get_header_type(bus, device, 0);
		if header_type & 0x80 != 0 {
			// It's a multi function device, check remaining functions
			for func in 1..8 {
				let vendor = get_vendor_id(bus, device, func);
				if vendor != 0xFFFF {
					check_function(bus, device, func)
				}
			}
		}
	}
}

fn check_function(bus: u8, device: u8, func: u8) {
	let vendor_id = get_vendor_id(bus, device, func);
	serial_println!("Vendor: {} ({:#X})", vendor_id, vendor_id);
	let class_code = get_class_code(bus, device, func);
	let subclass_code = get_subclass_code(bus, device, func);
	serial_println!("Class: {:#x} {:#X}", class_code, subclass_code);
}
