use alloc::{boxed::Box, vec::Vec};
use bitflags::bitflags;
use core::{
	fmt::{Debug, Display},
	marker::PhantomData,
	mem::{align_of, size_of},
	ops::{Deref, DerefMut},
};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use modular_bitfield::{bitfield, prelude::*};
use spin::Mutex;
use x86_64::{
	structures::{idt::InterruptDescriptorTable, paging::mapper::Translate},
	PhysAddr, VirtAddr,
};

use super::pci;
use crate::mem::{
	heap::{uncached_allocate_value, uncached_allocate_zeroed, UBox},
	paging::{self, virt_to_phys},
	volatile::V,
};

mod fis;
use fis::*;

mod disk;

const PRDTL: usize = 8;

lazy_static! {
	static ref PHYS_TO_VIRT: Mutex<HashMap<u64, VirtAddr>> = Mutex::new(HashMap::new());
}

/// Setup AHCI
pub unsafe fn setup() {
	// Get all PCI functions
	let functions = pci::recursive_scan();
	// Filter the function with the Mass Media - Sata class
	let result = functions
		.iter()
		.find(|func| pci::get_class_code(**func) == 0x01 && pci::get_subclass_code(**func) == 0x06);
	match result {
		Some(function) => {
			serial_println!("Found AHCI device: {:?}", function);
			let abar = pci::get_bars(*function)[5];
			serial_println!("ABAR: {:?}", abar);
			let interrupt = pci::get_interrupt(*function);
			serial_println!("Interrupt: {:?}", interrupt);
			let line = interrupt.line;

			unsafe {
				use crate::cpu::interrupts::IDT;
				let idt = &mut *(&mut *IDT.lock() as *mut InterruptDescriptorTable);
				idt[(40 + line).into()].set_handler_fn(interrupt_handler);
				idt.load();
				x86_64::instructions::interrupts::enable();
			}

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
			hba_memory = &mut *(virt_addr.as_mut_ptr());
			println!("{}", hba_memory.version.read());

			hba_memory.global_host_control.write(1 << 31 | 1 << 1);

			let ports = hba_memory.available_ports();
			for port in &ports {
				println!("{:?}", hba_memory.ports[*port].get_device_type());
			}

			for port in &ports {
				serial_println!("Rebase port {}", port);
				hba_memory.ports[*port].rebase();
			}
			const SECTORS: u64 = 8;
			let mut buf = UBox::new([[5; 512]; SECTORS as usize]);
			serial_println!("Read port 0");
			hba_memory.ports[ports[0]].read(0, &mut *buf);
			serial_println!("{:?}", *buf);
		}
		None => {
			serial_println!("No AHCI device, cannot access storage!");
		}
	}
}

type Sector = [u8; 512];

#[derive(Clone, Copy)]
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

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct Port {
	command_list_base: V<PhysPtr<CommandList>>,
	fis_base_address: V<PhysPtr<RecievedFis>>,
	interrupt_status: V<u32>,
	interrupt_enable: V<u32>,
	command_and_status: V<Status>,
	_reserved0: V<u32>,
	task_file_data: V<u32>,
	signature: V<u32>,
	sata_status: V<u32>,
	sata_control: V<u32>,
	sata_error: V<u32>,
	sata_active: V<u32>,
	command_issue: V<u32>,
	sata_notification: V<u32>,
	fis_based_switch_control: V<u32>,
	_reserved1: [u32; 11],
	vendor_specific: V<[u32; 4]>,
}

impl Port {
	unsafe fn read(&mut self, start_sector: u64, buf: &mut [Sector]) {
		let sector_count = buf.len();

		// Clear pending interrut bits
		serial_println!("Interrupt Status , Before clear: {:#X}", self.interrupt_status.read());
		self.interrupt_status.write(u32::MAX);
		serial_println!("Interrupt Status , After clear: {:#X}", self.interrupt_status.read());

		let slot_option = self.find_command_slot();
		if let Some(slot) = slot_option {
			let mut read = self.command_list_base.read();
			let command_list;
			command_list = &mut *read;
			let command_header: &mut CommandHeader = &mut command_list.0[slot];
			let bits = &mut command_header.bits;
			bits.write(bits.read().with_write(false));
			let cfl = (size_of::<FisRegHostToDevice>() / 4) as u8;
			bits.write(
				bits.read()
					.with_command_fis_length_checked(cfl)
					.expect("Command fis length out of bounds"),
			);
			command_header.prdt_length.write(1 as u16);
			let mut read = command_header.command_table_base.read();
			let command_table;
			command_table = &mut *read;
			(command_table as *mut CommandTable).write_bytes(0, 1);

			let entry = &mut command_table.prdt_entry[0];
			entry.data_base_address.write(PhysPtr::new(buf.as_mut_ptr()));

			entry
				.bits
				.write(entry.bits.read().with_byte_count(((sector_count << 9) - 1) as u32));
			entry.bits.write(entry.bits.read().with_interrupt_on_completion(true));

			let command_fis;
			command_fis = &mut command_table.command_fis as *mut _ as *mut FisRegHostToDevice;
			let bits = FisRegH2DBits::new().with_command_or_control(true);
			let command = 0x25;

			command_fis.write_volatile(FisRegHostToDevice::new(
				bits,
				command,
				0,
				start_sector,
				sector_count as u16,
				1 << 6,
			));

			let mut broke = false;
			for _ in 0..0x100000 {
				if self.task_file_data.read() & 0x88 == 0 {
					broke = true;
					break;
				}
			}
			if broke {
				let ci = 1 << slot;
				// Issue command
				serial_println!("Interrupt Status , Before command: {:#X}", self.interrupt_status.read());

				serial_println!("Issueing command");
				self.command_issue.write(ci);

				// serial_println!("Interrupt Status , After command: {:#X}", self.interrupt_status.read());

				// wait for completion
				let mut count = 0;
				loop {
					if self.command_issue.read() & ci == 0 {
						break;
					}
					if self.interrupt_status.read() & (1 << 30) != 0 {
						panic!("Read disk error");
						// TODO fail
					}
					count += 1;
				}
				if self.interrupt_status.read() & (1 << 30) != 0 {
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

	unsafe fn find_command_slot(&self) -> Option<usize> {
		let mut slots = self.command_issue.read() | self.sata_active.read();
		for i in 0..32 {
			if slots & 1 == 0 {
				return Some(i);
			}
			slots >>= 1;
		}
		None
	}

	unsafe fn get_interface_power_management(&self) -> u8 {
		((self.sata_status.read() >> 8) & 0x0F) as u8
	}

	unsafe fn get_device_detection(&self) -> u8 {
		(self.sata_status.read() & 0x0F) as u8
	}

	unsafe fn is_device_connected(&self) -> bool {
		self.get_device_detection() == 3 && self.get_interface_power_management() == 1
	}

	#[allow(dead_code)]
	unsafe fn get_device_type(&self) -> DeviceType {
		match self.signature.read() {
			0x0000_0101 => DeviceType::Sata,
			0xEB14_0101 => DeviceType::Satapi,
			0xc33c_0101 => DeviceType::Semb,
			0x9669_0101 => DeviceType::PortMultiplier,
			_ => DeviceType::Other,
		}
	}

	unsafe fn rebase(&mut self) {
		self.stop_command();

		let fis_base_address: *mut RecievedFis = uncached_allocate_zeroed();
		self.fis_base_address.write(PhysPtr::new(fis_base_address));
		// serial_println!("read after write {:?}", self.fis_base_address.read());

		let command_list_base: *mut CommandList = uncached_allocate_value(CommandList(
			[CommandHeader {
				bits: V::zeroed(),
				prdt_length: V::from(PRDTL as u16),
				prd_byte_count: V::zeroed(),
				_reserved: [0; 4],
				command_table_base: V::zeroed(),
			}; 32],
		));
		for command_header in &mut (*command_list_base).0 {
			command_header
				.command_table_base
				.write(PhysPtr::new(uncached_allocate_zeroed()));
		}

		self.command_list_base.write(PhysPtr::new(command_list_base));
		// serial_println!("read after write {:?}", self.command_list_base.read());

		// Clear interrupts
		self.interrupt_status.write(u32::MAX);
		// Enable interrupt
		self.interrupt_enable.write(1);
		self.start_command();
	}

	unsafe fn start_command(&mut self) {
		// wait until CR is cleared
		while self.command_and_status.read().contains(Status::COMMAND_LIST_RUNNING) {}

		// self.command_and_status.insert(Status::FIS_RECEIVED_ENABLE);
		// self.command_and_status.insert(Status::START);
		self.command_and_status
			.write(self.command_and_status.read() | Status::FIS_RECEIVED_ENABLE);
		self.command_and_status
			.write(self.command_and_status.read() | Status::START);
	}

	unsafe fn stop_command(&mut self) {
		let status = &mut self.command_and_status;

		status.write(status.read() - Status::START);
		status.write(status.read() - Status::FIS_RECEIVED_ENABLE);

		// wait until FR and CR are cleared
		while {
			let status_read = status.read();
			status_read.contains(Status::FIS_RECEIVED_RUNNING) || status_read.contains(Status::COMMAND_LIST_RUNNING)
		} {}
	}
}

#[derive(Debug)]
#[repr(C)]
struct Memory {
	capabilities: V<u32>,
	global_host_control: V<u32>,
	interrupt_status: V<u32>,
	port_implemented: V<u32>,
	version: V<AHCIVersion>,
	ccc_control: V<u32>, // Command comletion coalescing control
	ccc_ports: V<u32>,   // Command comletion coalescing ports
	enclosure_management_location: V<u32>,
	enclosure_management_control: V<u32>,
	capabilities_extended: V<u32>,
	bios_os_handoff: V<u32>,
	_reserved: [u8; 0xA0 - 0x2C],
	vendor_specific: [u8; 0x100 - 0xA0],
	ports: [Port; 32],
}

impl Memory {
	unsafe fn is_port_implemented(&self, port: u8) -> bool {
		(self.port_implemented.read() >> port) & 1 == 1
	}

	unsafe fn is_port_available(&self, port: u8) -> bool {
		{
			self.is_port_implemented(port) && self.ports[port as usize].is_device_connected()
		}
	}

	unsafe fn available_ports(&self) -> Vec<usize> {
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
	bits: V<CommandHeaderBits>,
	prdt_length: V<u16>, // Physical region descriptor table length in entries (should be equal to [PRDTL]
	prd_byte_count: V<u32>, // Physical region descriptor byte count transffered
	command_table_base: V<PhysPtr<CommandTable>>,
	_reserved: [u32; 4],
}

#[allow(dead_code)]
#[repr(align(1024))]
#[derive(Debug, Copy, Clone)]
struct CommandList([CommandHeader; 32]);

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
	data_base_address: V<PhysPtr<Sector>>,
	_reserved: V<u32>,
	bits: V<PrdtEntryBits>,
}

const _: () = {
	// Assert that the sizes and alignments of the types are correct
	assert!(size_of::<FisDmaSetup>() == 28);
	assert!(size_of::<FisPioSetup>() == 20);
	assert!(size_of::<FisRegDeviceToHost>() == 20);
	assert!(size_of::<FisSetDeviceBits>() == 8);
	assert!(size_of::<FisRegDeviceToHost>() == 20);
	assert!(size_of::<FisRegHostToDevice>() == 20);
	assert!(size_of::<Port>() == 0x80);
	assert!(size_of::<AHCIVersion>() == 0x4);
	assert!(size_of::<CommandTable>() == 256);
	assert!(align_of::<CommandTable>() == 128);
	assert!(size_of::<Memory>() == 0x1100);
	assert!(align_of::<CommandList>() == 1024);
	assert!(size_of::<CommandList>() == 1024);
	assert!(align_of::<RecievedFis>() == 256);
	assert!(size_of::<RecievedFis>() == 256);
	assert!(size_of::<FisDmaSetup>() == 28);
	assert!(size_of::<PrdtEntry>() == 16);
	assert!(size_of::<CommandHeaderBits>() == 2);
	assert!(size_of::<PrdtEntryBits>() == 4);
	{
		assert!(size_of::<PhysPtr<RecievedFis>>() == 8);
		assert!(size_of::<PhysPtr<CommandList>>() == 8);
		assert!(size_of::<PhysPtr<CommandTable>>() == 8);
	}
};
use crate::cpu::interrupts::PICS;
use x86_64::structures::idt::InterruptStackFrame;
extern "x86-interrupt" fn interrupt_handler(_stack_frame: InterruptStackFrame) {
	serial_println!("-------------------------------Interrupt-------------------------");
	unsafe {
		// TODO make this number depend on the PCI line register
		PICS.lock().notify_end_of_interrupt(42);
	}
}

#[repr(transparent)]
#[derive(Copy, Clone, Hash)]
struct PhysPtr<T> {
	addr: u64,
	phantom: PhantomData<T>,
}

// TODO probably replace this with a different way to solve the physical - virtual issue.
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
				// serial_println!("Create phys pointer {:#x} to virtual addr {:?}", phys, ptr);
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

impl<T> Debug for PhysPtr<T> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "{:#X}", self.addr)
	}
}

impl<T> Deref for PhysPtr<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		// serial_println!("Deref phys pointer {:#x}", self.addr);
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
		// serial_println!("Deref phys pointer {:#x}", self.addr);
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
