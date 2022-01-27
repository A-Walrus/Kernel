use alloc::{boxed::Box, vec::Vec};
use core::{
	fmt::{Debug, Display},
	marker::PhantomData,
	mem::{align_of, size_of},
	ops::{Deref, DerefMut},
};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::{structures::paging::mapper::Translate, PhysAddr, VirtAddr};

use super::pci;
use crate::mem::{
	heap::{uncached_allocate_value, uncached_allocate_zeroed, UBox},
	paging::{self, virt_to_phys},
};

use bitflags::bitflags;
use modular_bitfield::{bitfield, prelude::*};

// TODO I think the addresses are physical :( so alot of code is wrong

const PRDTL: usize = 8;

lazy_static! {
	static ref PHYS_TO_VIRT: Mutex<HashMap<u64, VirtAddr>> = Mutex::new(HashMap::new());
}

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

bitflags! {
	struct Status:u32 {
		const START = 0x0001;
		const FIS_RECEIVED_ENABLE = 0x0010;
		const FIS_RECEIVED_RUNNING = 0x4000;
		const COMMAND_LIST_RUNNING = 0x8000;

	}
}

#[derive(Debug)]
#[repr(C)]
struct Port {
	command_list_base: PhysPtr<CommandList>,
	fis_base_address: PhysPtr<RecievedFis>,
	interrupt_status: u32,
	interrupt_enable: u32,
	command_and_status: Status,
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

type Sector = [u8; 512];

impl Port {
	fn read(&mut self, start_sector: u64, buf: &mut [Sector]) {
		self.interrupt_enable = 0xFFFFFFFF;

		serial_println!("{}", self.interrupt_enable);

		let mut sector_count = buf.len();
		// Clear pending interrut bits
		serial_println!("Interrupt Status , Before clear: {:#X}", self.interrupt_status);
		self.interrupt_status = 0xFFFFFFFF;
		serial_println!("Interrupt Status , After clear: {:#X}", self.interrupt_status);

		let slot_option = self.find_command_slot();
		if let Some(slot) = slot_option {
			let command_list;
			command_list = &mut *self.command_list_base;
			let command_header: &mut CommandHeader = &mut command_list.0[slot];
			let bits = &mut command_header.bits;
			bits.set_write(false);
			let cfl = (size_of::<FisRegHostToDevice>() / 4) as u8;
			bits.set_command_fis_length_checked(cfl)
				.expect("Command fis length out of bounds");
			command_header.prdt_length = 1 as u16;
			let command_table;
			unsafe {
				command_table = &mut *command_header.command_table_base;
				(command_table as *mut CommandTable).write_bytes(0, 1);
			}

			let entry = &mut command_table.prdt_entry[0];
			entry.data_base_address = virt_to_phys(VirtAddr::new(buf.as_mut_ptr() as u64)).as_u64();
			entry.bits.set_byte_count(((sector_count << 9) - 1) as u32);
			entry.bits.set_interrupt_on_completion(true);

			let command_fis;
			unsafe {
				command_fis = &mut *(&mut command_table.command_fis as *mut _ as *mut FisRegHostToDevice);
			}
			let bits = FisRegH2DBits::new().with_command_or_control(true);
			let command = 0x25;
			*command_fis = FisRegHostToDevice::new(bits, command, 0, start_sector, sector_count as u16, 1 << 6);

			let mut broke = false;
			for _ in 0..0x100000 {
				if self.task_file_data & 0x88 == 0 {
					broke = true;
					break;
				}
			}
			if broke {
				let ci = 1 << slot;
				// Issue command
				serial_println!("Interrupt Status , Before command: {:#X}", self.interrupt_status);
				self.command_issue = ci;

				serial_println!("Interrupt Status , After command: {:#X}", self.interrupt_status);

				// wait for completion
				let mut count = 0;
				loop {
					if self.command_issue & ci == 0 {
						break;
					}
					if self.interrupt_status & (1 << 30) != 0 {
						panic!("Read disk error");
						// TODO fail
					}
					count += 1;
				}
				serial_println!("count: {}", count);
				if self.interrupt_status & (1 << 30) != 0 {
					panic!("Read disk error");
					// TODO fail
				}
			} else {
				panic!("Port is hung");
				// TODO fail
			}
		} else {
			panic!("No command slots");
			// TODO fail
		}
	}

	fn find_command_slot(&self) -> Option<usize> {
		let mut slots = self.command_issue | self.sata_active;
		for i in 0..32 {
			if slots & 1 == 0 {
				return Some(i);
			}
			slots >>= 1;
		}
		None
	}

	fn get_interface_power_management(&self) -> u8 {
		((self.sata_status >> 8) & 0x0F) as u8
	}

	fn get_device_detection(&self) -> u8 {
		(self.sata_status & 0x0F) as u8
	}

	fn is_device_connected(&self) -> bool {
		self.get_device_detection() == 3 && self.get_interface_power_management() == 1
	}

	#[allow(dead_code)]
	fn get_device_type(&self) -> DeviceType {
		match self.signature {
			0x0000_0101 => DeviceType::Sata,
			0xEB14_0101 => DeviceType::Satapi,
			0xc33c_0101 => DeviceType::Semb,
			0x9669_0101 => DeviceType::PortMultiplier,
			_ => DeviceType::Other,
		}
	}

	fn rebase(&mut self) {
		self.stop_command();

		unsafe {
			let fis_base_address: *mut RecievedFis = uncached_allocate_zeroed();
			self.fis_base_address = PhysPtr::new(fis_base_address);

			let command_list_base: *mut CommandList = uncached_allocate_value(CommandList(
				[CommandHeader {
					bits: CommandHeaderBits::new(),
					prdt_length: PRDTL as u16,
					prd_byte_count: 0,
					_reserved: [0; 4],
					command_table_base: PhysPtr::new(uncached_allocate_zeroed()),
				}; 32],
			));
			self.command_list_base = PhysPtr::new(command_list_base);
		}

		self.start_command();
	}

	fn start_command(&mut self) {
		// wait until CR is cleared
		while self.command_and_status.contains(Status::COMMAND_LIST_RUNNING) {}

		self.command_and_status.insert(Status::FIS_RECEIVED_ENABLE);
		self.command_and_status.insert(Status::START);
	}

	fn stop_command(&mut self) {
		let status = &mut self.command_and_status;

		status.remove(Status::START);
		status.remove(Status::FIS_RECEIVED_ENABLE);

		// wait until FR and CR are cleared
		while status.contains(Status::FIS_RECEIVED_RUNNING) || status.contains(Status::COMMAND_LIST_RUNNING) {}
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

	fn is_port_available(&self, port: u8) -> bool {
		self.is_port_implemented(port) && self.ports[port as usize].is_device_connected()
	}

	fn available_ports(&self) -> Vec<usize> {
		(0..32).filter(|i| self.is_port_available(*i as u8)).collect()
	}
}

#[bitfield]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
struct CommandHeaderBits {
	command_fis_length: B5,
	atapi: bool,
	/// true: H2D, false : D2H
	write: bool,
	prefetchable: bool,
	reset: bool,
	bist: bool,
	clear_busy_upon_r_ok: bool,
	reserved: B1,
	port_multiplier_port: B4,
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct CommandHeader {
	bits: CommandHeaderBits,
	prdt_length: u16,    // Physical region descriptor table length in entries (should be equal to [PRDTL]
	prd_byte_count: u32, // Physical region descriptor byte count transffered
	command_table_base: PhysPtr<CommandTable>,
	_reserved: [u32; 4],
}

#[allow(dead_code)]
#[repr(align(1024))]
#[derive(Debug)]
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

#[bitfield]
#[derive(Debug)]
struct FisRegH2DBits {
	port_multiplier_port: B4,
	reserved: B3,
	/// true: command, false: control
	command_or_control: bool,
}

#[repr(C)]
#[derive(Debug)]
struct FisRegHostToDevice {
	fis_type: FisType,
	bits: FisRegH2DBits,
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
	countl: u8,
	counth: u8,
	_reserved0: u8,
	control: u8,
	_reserved1: [u8; 4],
}

impl FisRegHostToDevice {
	fn new(bits: FisRegH2DBits, command: u8, control: u8, lba: u64, count: u16, device: u8) -> Self {
		Self {
			fis_type: FisType::RegHostToDevice,
			bits,
			command,
			feature_low: 0,
			lba0: lba as u8,
			lba1: (lba >> 8) as u8,
			lba2: (lba >> 16) as u8,
			lba3: (lba >> 24) as u8,
			lba4: (lba >> 32) as u8,
			lba5: (lba >> 40) as u8,
			feature_high: 0,
			countl: count as u8,
			counth: (count >> 8) as u8,
			_reserved0: 0,
			_reserved1: [0; 4],
			control,
			device,
		}
	}
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

#[derive(Copy, Clone, Debug)]
#[repr(C, align(128))]
struct CommandTable {
	command_fis: [u8; 64],
	atapi_command: [u8; 16],
	_reserved: [u8; 48],
	prdt_entry: [PrdtEntry; PRDTL],
}

#[bitfield]
#[derive(Debug, Clone, Copy)]
struct PrdtEntryBits {
	/// Byte count *minus 1*
	byte_count: B22,
	reserved: B9,
	interrupt_on_completion: bool,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
struct PrdtEntry {
	data_base_address: u64, // TODO figure out type TO_PHYS
	_reserved: u32,
	bits: PrdtEntryBits,
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
	_bits0: u8,
	_bits1: u8,
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
	assert!(size_of::<FisDmaSetup>() == 28);
	assert!(size_of::<FisPioSetup>() == 20);
	assert!(size_of::<FisRegDeviceToHost>() == 20);
	assert!(size_of::<FisSetDeviceBits>() == 8);
	assert!(size_of::<Port>() == 0x80);
	assert!(size_of::<AHCIVersion>() == 0x4);
	assert!(size_of::<CommandTable>() == 256);
	assert!(size_of::<Memory>() == 0x1100);
	assert!(align_of::<CommandList>() == 1024);
	assert!(size_of::<CommandList>() == 1024);
	assert!(align_of::<RecievedFis>() == 256);
	assert!(size_of::<RecievedFis>() == 256);
	assert!(size_of::<FisRegDeviceToHost>() == 20);
	assert!(size_of::<FisRegHostToDevice>() == 20);
	assert!(size_of::<FisDmaSetup>() == 28);
	assert!(size_of::<PrdtEntry>() == 16);
	assert!(size_of::<CommandHeaderBits>() == 2);
	assert!(size_of::<PrdtEntryBits>() == 4);
	{
		assert!(size_of::<PhysPtr<RecievedFis>>() == 8);
		assert!(size_of::<PhysPtr<CommandList>>() == 8);
		assert!(size_of::<PhysPtr<CommandTable>>() == 8);
	}
	assert!(align_of::<CommandTable>() == 128);
};

use x86_64::structures::idt::InterruptStackFrame;
extern "x86-interrupt" fn interrupt_handler(stack_frame: InterruptStackFrame) {
	serial_println!("Interrupt!!!!! ");
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
			let line = pci::get_interrupt(*function).line;

			use crate::cpu::interrupts::IDT;
			IDT.lock()[line.into()].set_handler_fn(interrupt_handler);

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
			let ports = hba_memory.available_ports();
			for port in &ports {
				hba_memory.ports[*port].rebase();
			}
			const SECTORS: u64 = 8;
			let mut buf = UBox::new([[5; 512]; SECTORS as usize]);
			hba_memory.ports[ports[0]].read(0, &mut *buf);
			for _ in 0..0x100 {
				serial_print!(".");
				use x86_64::instructions::hlt;
				hlt();
			}
			serial_println!("{:?}", *buf);
		}
		None => {
			serial_println!("No AHCI device, cannot access storage!");
		}
	}
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Hash)]
struct PhysPtr<T> {
	addr: u64,
	phantom: PhantomData<T>,
}

impl<T> PhysPtr<T> {
	fn new(ptr: *mut T) -> Self {
		let virt = VirtAddr::new(ptr as usize as u64);

		let table;
		unsafe {
			table = paging::get_offset_page_table(paging::get_current_page_table());
		}
		let result = table.translate_addr(virt);
		match result {
			Some(phys) => {
				let phys = phys.as_u64();
				PHYS_TO_VIRT.lock().insert(phys, virt);
				Self {
					addr: phys,
					phantom: PhantomData,
				}
			}
			_ => {
				unreachable!("This should be mapped and translatable")
			}
		}
	}
}

impl<T> Deref for PhysPtr<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		let phys = self.addr;
		let lock = PHYS_TO_VIRT.lock();
		let option_virt = lock.get(&phys);
		match option_virt {
			Some(v) => unsafe { &*v.as_ptr() },
			None => {
				unreachable!("All physical pointers should be in the map")
			}
		}
	}
}

impl<T> DerefMut for PhysPtr<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		let phys = self.addr;
		let lock = PHYS_TO_VIRT.lock();
		let option_virt = lock.get(&phys);
		match option_virt {
			Some(v) => unsafe { &mut *v.as_mut_ptr() },
			None => {
				unreachable!("All physical pointers should be in the map")
			}
		}
	}
}
