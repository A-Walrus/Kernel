use core::{
	fmt::{Debug, Display},
	mem::{align_of, size_of},
};

use super::pci;
use crate::mem::{
	heap::{self, ALLOCATOR, UNCACHED_ALLOCATOR},
	paging,
};

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
enum DeviceType {
	Sata,
	Semb,
	PortMultiplier,
	Satapi,
	Other,
}

#[derive(Debug)]
#[repr(C)]
struct Port {
	command_list_base: u64, // base address 1KiB aligned
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
	_reserved1: [u32; 11],
	vendor_specific: [u32; 4],
}

impl Port {
	fn get_interface_power_management(&self) -> u8 {
		((self.sata_status >> 8) & 0x0F) as u8
	}

	fn get_device_detection(&self) -> u8 {
		(self.sata_status & 0x0F) as u8
	}

	fn get_device_type(&self) -> DeviceType {
		match self.signature {
			0x0000_0101 => DeviceType::Sata,
			0xEB14_0101 => DeviceType::Satapi,
			0xc33c_0101 => DeviceType::Semb,
			0x9669_0101 => DeviceType::PortMultiplier,
			_ => DeviceType::Other,
		}
	}
}

#[derive(Debug)]
#[repr(C)]
struct Memory {
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
	ports: [Port; 32],
}

impl Memory {
	fn is_port_implemented(&self, port: u8) -> bool {
		(self.port_implemented >> port) & 1 == 1
	}
}

#[derive(Debug)]
#[repr(C)]
struct CommandHeader {
	_bits: u16,
	prdt_length: u16,    // Physical region descriptor table length in entries
	prd_byte_count: u32, // Physical region descriptor byte count transffered
	command_table_base: u32,
	command_table_base_upper: u32,
	_reserved: [u32; 4],
}

#[allow(dead_code)]
#[repr(align(1024))]
struct CommandList([CommandHeader; 32]);

#[allow(dead_code)]
#[derive(Debug)]
#[repr(u8)]
enum FisType {
	RegHostToDevice = 0x27,
	RegDeviceToHost = 0x34,
	DmaActivate = 0x39,
	DmaSetup = 0x41,
	Data = 0x46,
	Bist = 0x58,
	PioSetup = 0x5F,
	DeviceBits = 0xA1,
}

#[repr(C)]
#[derive(Debug)]
struct FisRegDeviceToHost {
	fis_type: FisType,
	_bits: u8,
	status: u8,
	error: u8,
	lba0: u8,
	lba1: u8,
	lba2: u8,
	device: u8,
	lba3: u8,
	lba4: u8,
	lba5: u8,
	_reserved0: u8,
	count_low: u8,
	count_hight: u8,
	_reserved1: [u8; 2],
	_reserved2: [u8; 4],
}

#[repr(C)]
#[derive(Debug)]
struct FisRegHostToDevice {
	fis_type: FisType,
	_bits: u8,
	command: u8,
	feature_low: u8,
	lba0: u8,
	lba1: u8,
	lba2: u8,
	device: u8,
	lba3: u8,
	lba4: u8,
	lba5: u8,
	feature_high: u8,
	count_low: u8,
	count_high: u8,
	_reserved0: u8,
	control: u8,
	_reserved1: [u8; 4],
}

#[repr(C)]
#[derive(Debug)]
struct FisData {
	fis_type: FisType,
	_bits: u8,
	_rserved0: [u8; 2],
	data: (), // TODO figure out what type this should be
}

#[repr(C)]
#[derive(Debug)]
struct FisPioSetup {
	fis_type: FisType,
	_bits: u8,
	status: u8,
	error: u8,
	lba0: u8,
	lba1: u8,
	lba2: u8,
	device: u8,
	lba3: u8,
	lba4: u8,
	lba5: u8,
	_reserved0: u8,
	count_low: u8,
	count_high: u8,
	_reserved1: u8,
	e_status: u8,
	transfer_count: u16,
	_reserved2: [u8; 2],
}

#[derive(Debug)]
#[repr(C, align(128))]
struct CommandTable {
	command_fis: [u8; 64],
	atapi_command: [u8; 16],
	_reserved: [u8; 48],
	prdt_entries: (), // TODO figure out type for this. Its length is command header prdt_length
}

#[derive(Debug)]
#[repr(C)]
struct PrdtEntry {
	data_base_address: u32,
	data_base_address_upper: u32,
	_reserved: u32,
	_bits: u32,
}
#[derive(Debug)]
#[repr(C)]
struct FisDmaSetup {
	fis_type: FisType,
	_bits: u8,
	_reserved0: [u8; 2],
	dma_buffer_id_low: u32,
	dma_buffer_id_high: u32,
	_reserved1: u32,
	dma_buffer_offset: u32,
	transfer_count: u32,
	_reserved2: [u8; 4],
}

#[derive(Debug)]
#[repr(C)]
struct FisSetDeviceBits {
	fis_type: FisType,
	_bits: u16,
	error: u8,
	_reserved: [u8; 4],
}
#[derive(Debug)]
#[repr(C, align(256))]
struct RecievedFis {
	dma_setup: FisDmaSetup,
	_pad0: [u8; 4],
	pio_setup: FisPioSetup,
	_pad1: [u8; 12],
	d2h_register: FisRegDeviceToHost,
	_pad2: [u8; 4],
	set_device_bits: FisSetDeviceBits,
	unknown_fis: [u8; 64],
	_reserved: [u8; 0x100 - 0xA0],
}

const _: () = {
	assert!(size_of::<Port>() == 0x80);
	assert!(size_of::<AHCIVersion>() == 0x4);
	assert!(size_of::<Memory>() == 0x1100);
	assert!(align_of::<CommandList>() == 1024);
	assert!(align_of::<RecievedFis>() == 256);
	assert!(size_of::<FisRegDeviceToHost>() == 20);
	assert!(size_of::<FisRegHostToDevice>() == 20);
	assert!(size_of::<FisDmaSetup>() == 28);
	assert!(size_of::<PrdtEntry>() == 16);
	assert!(align_of::<u64>() == 8);
	assert!(align_of::<CommandTable>() == 128);
};

/// Setup AHCI
pub fn setup() {
	let boxed = heap::uncache_box_new(5);

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
			let hba_memory: &mut Memory;
			unsafe {
				hba_memory = &mut *(virt_addr.as_mut_ptr());
			}
			println!("{}", hba_memory.version);
			probe_ports(hba_memory);
		}
		None => {
			serial_println!("No AHCI device, cannot access storage!");
		}
	}
}

fn probe_ports(abar: &Memory) {
	for port in 0..32 {
		if abar.is_port_implemented(port) {
			let device_type = check_type(&abar.ports[port as usize]);
			serial_println!("Port {}: {:?}", port, device_type);
		}
	}
}

fn check_type(port: &Port) -> Option<DeviceType> {
	if port.get_device_detection() != 3 || port.get_interface_power_management() != 1 {
		// no device connected
		return None;
	} else {
		// device connected
		serial_println!("{:#X}", port.command_list_base);
		Some(port.get_device_type())
	}
}
