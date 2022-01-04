use x86_64::instructions::port::{Port, PortGeneric, ReadOnlyAccess, ReadWriteAccess, WriteOnlyAccess};

const CONFIG_ADDRESS: PortGeneric<u32, WriteOnlyAccess> = PortGeneric::new(0xCF8);
const CONFIG_DATA: PortGeneric<u32, ReadOnlyAccess> = PortGeneric::new(0xCFC);
use alloc::vec::Vec;

use crate::{serial_print, serial_println};

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

/// test pci
pub fn testing() {
	let res = brute_force_scan();
	serial_println!("{:?}", res);
	for func in res {
		func_info(func);
	}
}

#[derive(Debug, Copy, Clone)]
struct Function {
	bus: u8,
	slot: u8,
	function: u8,
}

// Read a word at a certain bus, slot, func, offset
fn pci_config_read(func: Function, register: u8) -> u32 {
	let lbus = func.bus as u32;
	let lslot = func.slot as u32;
	let lfunc = func.function as u32;
	let lregister = register as u32;

	let address: u32 = lbus << 16 | lslot << 11 | lfunc << 8 | lregister << 2 | 0x80000000;

	unsafe {
		CONFIG_ADDRESS.write(address);
	}

	unsafe { CONFIG_DATA.read() }
}

fn brute_force_scan() -> Vec<Function> {
	let mut vec = Vec::new();
	for bus in 0..=255 {
		for device in 0..32 {
			scan_device(bus, device, &mut vec);
		}
	}
	vec
}

fn get_vendor_id(func: Function) -> u16 {
	let reg = pci_config_read(func, 0);
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

fn get_header_type(func: Function) -> HeaderType {
	match get_header_type_val(func) & 0b0111_1111 {
		0x0 => HeaderType::Regular,
		0x1 => HeaderType::PciToPci,
		0x2 => HeaderType::PciToCardBus,
		_ => HeaderType::Reserved,
	}
}

fn get_header_type_val(func: Function) -> u8 {
	let reg = pci_config_read(func, 3);
	(reg >> 16) as u8
}

fn get_class_code(func: Function) -> u8 {
	let reg = pci_config_read(func, 2);
	(reg >> 24) as u8
}

fn get_subclass_code(func: Function) -> u8 {
	let reg = pci_config_read(func, 2);
	(reg >> 16) as u8
}

#[derive(Debug)]
struct Interrupt {
	pin: u8,
	line: u8,
}
fn get_interrupt(func: Function) -> Interrupt {
	let reg = pci_config_read(func, 2);
	Interrupt {
		pin: (reg >> 8) as u8,
		line: reg as u8,
	}
}

type Bars = [u32; 6];

// Only correct if Header Type is Regular (0x0)
fn get_bars(func: Function) -> Bars {
	let mut bars = [0; 6];
	for i in 0u8..6 {
		bars[i as usize] = pci_config_read(func, 4 + i);
	}
	bars
}

fn scan_device(bus: u8, device: u8, found: &mut Vec<Function>) {
	let mut func = Function {
		bus,
		slot: device,
		function: 0,
	};
	if get_vendor_id(func) == 0xFFFF {
		// Device doesn't exist
	} else {
		// Device exists
		found.push(func);
		let header_type = get_header_type_val(func);
		if header_type & 0x80 != 0 {
			// It's a multi function device, check remaining functions
			for function in 1..8 {
				func.function = function;
				let vendor = get_vendor_id(func);
				if vendor != 0xFFFF {
					found.push(func);
				}
			}
		}
	}
}

fn func_info(func: Function) {
	let vendor_id = get_vendor_id(func);
	serial_println!("Vendor: {} ({:#X})", vendor_id, vendor_id);
	let class_code = get_class_code(func);
	let subclass_code = get_subclass_code(func);
	serial_println!("Class: {:#x} {:#X}", class_code, subclass_code);
	let header_type = get_header_type(func);
	serial_println!("Header Type: {:?}", header_type);
	let interrupt = get_interrupt(func);
	serial_println!("Interrupt: {:?}", interrupt);
	let bars = get_bars(func);
	serial_println!("Bars: {:?}", bars);

	serial_println!("");
}
