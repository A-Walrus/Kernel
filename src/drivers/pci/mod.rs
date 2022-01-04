use x86_64::instructions::port::{Port, PortGeneric, ReadOnlyAccess, ReadWriteAccess, WriteOnlyAccess};

const CONFIG_ADDRESS: PortGeneric<u32, WriteOnlyAccess> = PortGeneric::new(0xCF8);
const CONFIG_DATA: PortGeneric<u32, ReadOnlyAccess> = PortGeneric::new(0xCFC);

use crate::{serial_print, serial_println};

/// test pci
pub fn testing() {
	brute_force_scan();
}

// Config address format
// ╔════════╦════════════╦════════════╦════════════╦══════════════╦═════════════════╗
// ║ Bit 31 ║ Bits 30-24 ║ Bits 23-16 ║ Bits 15-11 ║   Bits 10-8  ║     Bits 7-0    ║
// ╠════════╬════════════╬════════════╬════════════╬══════════════╬═════════════════╣
// ║ Enable ║  Reserved  ║   Bus Num  ║ Device Num ║ Function Num ║ Register Offset ║
// ╚════════╩════════════╩════════════╩════════════╩══════════════╩═════════════════╝

// PCI header Start
// ╔════════════╦═══════════════╦═══════════════╦═════════════════╦════════════════════════╗
// ║  Register  ║   Bits 31-24  ║   Bits 23-16  ║    Bits 15-8    ║        Bits 7-0        ║
// ╠════════════╬═══════════════╩═══════════════╬═════════════════╩════════════════════════╣
// ║     0x0    ║           Device ID           ║                 Vendor ID                ║
// ╠════════════╬═══════════════════════════════╬══════════════════════════════════════════╣
// ║     0x1    ║             Status            ║                  Command                 ║
// ╠════════════╬═══════════════╦═══════════════╬═════════════════╦════════════════════════╣
// ║     0x2    ║   Class code  ║    Subclass   ║     Prog IF     ║       Revision ID      ║
// ╠════════════╬═══════════════╬═══════════════╬═════════════════╬════════════════════════╣
// ║     0x3    ║      BIST     ║  Header type  ║  Latency Timer  ║     Cache Line Size    ║
// ╚════════════╩═══════════════╩═══════════════╩═════════════════╩════════════════════════╝
// Rest of header for Regular type
// ╔════════════╦══════════════════════════════════════════════════════════════════════════╗
// ║     0x4    ║                          Base address #0 (BAR0)                          ║
// ╠════════════╬══════════════════════════════════════════════════════════════════════════╣
// ║     0x5    ║                          Base address #1 (BAR1)                          ║
// ╠════════════╬══════════════════════════════════════════════════════════════════════════╣
// ║     0x6    ║                          Base address #2 (BAR2)                          ║
// ╠════════════╬══════════════════════════════════════════════════════════════════════════╣
// ║     0x7    ║                          Base address #3 (BAR3)                          ║
// ╠════════════╬══════════════════════════════════════════════════════════════════════════╣
// ║     0x8    ║                          Base address #4 (BAR4)                          ║
// ╠════════════╬══════════════════════════════════════════════════════════════════════════╣
// ║     0x9    ║                          Base address #5 (BAR5)                          ║
// ╠════════════╬══════════════════════════════════════════════════════════════════════════╣
// ║     0xA    ║                            Cardbus CIS Pointer                           ║
// ╠════════════╬═══════════════════════════════╦══════════════════════════════════════════╣
// ║     0xB    ║          Subsystem ID         ║            Subsystem Vendor ID           ║
// ╠════════════╬═══════════════════════════════╩══════════════════════════════════════════╣
// ║     0xC    ║                        Expansion ROM base address                        ║
// ╠════════════╬═════════════════════════════════════════════════╦════════════════════════╣
// ║     0xD    ║                     Reserved                    ║  Capabilities Pointer  ║
// ╠════════════╬═════════════════════════════════════════════════╩════════════════════════╣
// ║     0xE    ║                                 Reserved                                 ║
// ╠════════════╬═══════════════╦═══════════════╦═════════════════╦════════════════════════╣
// ║     0xF    ║  Max latency  ║   Min Grant   ║  Interrupt PIN  ║     Interrupt Line     ║
// ╚════════════╩═══════════════╩═══════════════╩═════════════════╩════════════════════════╝

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

#[derive(Debug)]
#[repr(u8)]
enum HeaderType {
	Regular = 0x0,
	PciToPci = 0x1,
	PciToCardBus = 0x2,
	Reserved = 0xff,
}

fn get_header_type_enum(bus: u8, device: u8, func: u8) -> HeaderType {
	match get_header_type_num(bus, device, func) & 0b0111_1111 {
		0x0 => HeaderType::Regular,
		0x1 => HeaderType::PciToPci,
		0x2 => HeaderType::PciToCardBus,
		_ => HeaderType::Reserved,
	}
}

fn get_header_type_num(bus: u8, device: u8, func: u8) -> u8 {
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

#[derive(Debug)]
struct Interrupt {
	pin: u8,
	line: u8,
}
fn get_interrupt(bus: u8, device: u8, func: u8) -> Interrupt {
	let reg = pci_config_read(bus, device, func, 2);
	Interrupt {
		pin: (reg >> 8) as u8,
		line: reg as u8,
	}
}

fn check_device(bus: u8, device: u8) {
	if get_vendor_id(bus, device, 0) == 0xFFFF {
		// Device doesn't exist
	} else {
		// Device exists
		check_function(bus, device, 0);
		let header_type = get_header_type_num(bus, device, 0);
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
	let header_type = get_header_type_enum(bus, device, func);
	serial_println!("Header Type: {:?}", header_type);
	let interrupt = get_interrupt(bus, device, func);
	serial_println!("Interrupt: {:?}", interrupt);

	serial_println!("");
}
