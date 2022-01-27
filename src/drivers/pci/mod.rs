use x86_64::{
	instructions::port::{PortGeneric, ReadWriteAccess, WriteOnlyAccess},
	PhysAddr,
};

const CONFIG_ADDRESS: PortGeneric<u32, WriteOnlyAccess> = PortGeneric::new(0xCF8);
const CONFIG_DATA: PortGeneric<u32, ReadWriteAccess> = PortGeneric::new(0xCFC);
use alloc::vec::Vec;

use crate::serial_println;

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

/// Test pci
pub fn testing() {
	let res = recursive_scan();
	serial_println!("{:?}", res);
	for func in res {
		func_info(func);
	}
}

/// Struct representing a PCI function. This is like the PCI "address" of a function (thing that does
/// something). Some examples are network cards, storage cards, bus bridges, and so on.
#[derive(Debug, Copy, Clone)]
pub struct Function {
	/// The bus that this function is on (0-256)
	pub bus: u8,
	/// The slot/device that this function is on (0-32)
	pub slot: u8,
	/// The function that this is on the device (0-8)
	pub function: u8,
}

impl Function {
	fn new(bus: u8, slot: u8, function: u8) -> Self {
		Self { bus, function, slot }
	}
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

/// Recursively scan PCI buses and find all available functions. This starts at the ['function']
/// (0,0,0) and through buses and bridges finds the rest of the functions recursively.
pub fn recursive_scan() -> Vec<Function> {
	let mut found = Vec::new();
	let mut function = Function::new(0, 0, 0);

	let header_type = get_header_type(function);
	if header_type.multi_function {
		// multi function device
		for func in 0..8 {
			function.function = func;
			if get_vendor_id(function) != 0xFFFF {
				// function exists
				scan_bus(func, &mut found);
			}
		}
	} else {
		// single function device
		scan_bus(0, &mut found);
	}

	found
}

fn scan_bus(bus: u8, found: &mut Vec<Function>) {
	for device in 0..32 {
		scan_device(bus, device, found);
	}
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

#[allow(dead_code)]
#[derive(Debug)]
struct HeaderTypeField {
	header_type: HeaderType,
	multi_function: bool,
}

fn get_header_type(func: Function) -> HeaderTypeField {
	let val = get_header_type_val(func);
	HeaderTypeField {
		header_type: match val & 0b0111_1111 {
			0x0 => HeaderType::Regular,
			0x1 => HeaderType::PciToPci,
			0x2 => HeaderType::PciToCardBus,
			_ => HeaderType::Reserved,
		},
		multi_function: val & 0x80 != 0,
	}
}

fn get_header_type_val(func: Function) -> u8 {
	let reg = pci_config_read(func, 3);
	(reg >> 16) as u8
}

/// Get the class code/id of the function. Each class code represents a different type of device
/// storage, network, display...
pub fn get_class_code(func: Function) -> u8 {
	let reg = pci_config_read(func, 2);
	(reg >> 24) as u8
}

/// Get the subclass code/id of the function. Each subclass code represents a more specific
/// type for the class according to ['get_class_code'].
pub fn get_subclass_code(func: Function) -> u8 {
	let reg = pci_config_read(func, 2);
	(reg >> 16) as u8
}

fn get_secondary_bus(func: Function) -> u8 {
	let reg = pci_config_read(func, 6);
	(reg >> 8) as u8
}

#[allow(dead_code)]
#[derive(Debug)]
/// Struct describing the interrupt data in the PCI register
pub struct Interrupt {
	/// The interrupt pin (not sure what that means)
	pub pin: u8,
	/// The interrupt line (I believe that's the index IRQ)
	pub line: u8,
}

/// Get the interrupt data for the function
pub fn get_interrupt(func: Function) -> Interrupt {
	let reg = pci_config_read(func, 2);
	Interrupt {
		pin: (reg >> 8) as u8,
		line: reg as u8,
	}
}

// Memory space BAR layout
// ╔════════════════════════════════╦════════════════╦════════════╦════════════╗
// ║            Bits 31-4           ║      Bit 3     ║  Bits 2-1  ║    Bit 0   ║
// ╠════════════════════════════════╬════════════════╬════════════╬════════════╣
// ║  16-Byte Aligned Base Address  ║  Prefetchable  ║    Type    ║  Always 0  ║
// ╚════════════════════════════════╩════════════════╩════════════╩════════════╝
//
// IO space BAR layout
// ╔═══════════════════════════════╦════════════╦════════════╗
// ║           Bits 31-2           ║    Bit 1   ║    Bit 0   ║
// ╠═══════════════════════════════╬════════════╬════════════╣
// ║  4-Byte Aligned Base Address  ║  Reserved  ║  Always 1  ║
// ╚═══════════════════════════════╩════════════╩════════════╝

/// Enum representing a Base Address Register. Can either be in memory space or io space.
#[derive(Debug, Copy, Clone)]
pub enum Bar {
	/// Base Address register in memory space. Memory spase BARS are:
	/// - Located in physical ram.
	/// - Aligned to 16 Bytes
	MemorySpace {
		/// Whether reading this BAR has any side effects
		prefetchable: bool,
		/// The base address, this address is a physical address in ram
		base_address: PhysAddr,
	},
	/// Base Address register in I/O space. Its address is not within the physical ram.
	IOSpace {
		/// The I/O base address. It is 4 Byte aligned.
		base_address: PhysAddr,
	},
}

/// Get vector of BARS of this function. Only valid if Header Type is Regular (0x0)!
pub fn get_bars(func: Function) -> Vec<Bar> {
	let mut vec = Vec::new();
	let mut i = 0;
	while i < 6 {
		let bar_i = pci_config_read(func, 4 + i);
		let bar = {
			if bar_i % 2 == 0 {
				// Memory space
				let bar_type = (bar_i & 0b110) >> 1;
				let prefetchable = bar_i & 0b1000 != 0;
				match bar_type {
					0 => {
						// 32 bit
						Bar::MemorySpace {
							prefetchable,
							base_address: PhysAddr::new((bar_i & 0xFFFFFFF0) as u64),
						}
					}
					2 => {
						// 64 bit
						i += 1;
						let start = bar_i as u64;
						let end = pci_config_read(func, 4 + i) as u64;
						Bar::MemorySpace {
							prefetchable,
							base_address: PhysAddr::new((start & 0xFFFFFFF0) + ((end & 0xFFFFFFFF) << 32)),
						}
					}
					_ => {
						// Invalid
						unreachable!()
					}
				}
			} else {
				// IO space
				Bar::IOSpace {
					base_address: PhysAddr::new((bar_i & 0xFFFFFFFC) as u64),
				}
			}
		};
		vec.push(bar);
		i += 1;
	}
	vec
}

fn scan_device(bus: u8, device: u8, found: &mut Vec<Function>) {
	let mut func = Function::new(bus, device, 0);
	if get_vendor_id(func) == 0xFFFF {
		// Device doesn't exist
	} else {
		// Device exists
		scan_function(func, found);
		let header_type = get_header_type(func);
		if header_type.multi_function {
			// It's a multi function device, check remaining functions
			for function in 1..8 {
				func.function = function;
				let vendor = get_vendor_id(func);
				if vendor != 0xFFFF {
					scan_function(func, found);
				}
			}
		}
	}
}

fn scan_function(func: Function, found: &mut Vec<Function>) {
	found.push(func);
	let class_code = get_class_code(func);
	let subclass_code = get_subclass_code(func);
	if class_code == 0x6 && subclass_code == 0x4 {
		let secondary_bus = get_secondary_bus(func);
		scan_bus(secondary_bus, found);
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
	serial_println!("Bars [{}]: {:?}", bars.len(), bars);

	serial_println!("");
}
