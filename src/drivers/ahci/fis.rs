use crate::mem::volatile::V;
use bitflags::bitflags;
use modular_bitfield::{bitfield, prelude::*};

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum FisType {
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
#[derive(Debug, Copy, Clone)]
pub struct FisRegDeviceToHost {
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
pub struct FisRegH2DBits {
	pub port_multiplier_port: B4,
	reserved: B3,
	/// true: command, false: control
	pub command_or_control: bool,
}

#[repr(C)]
#[derive(Debug)]
pub struct FisRegHostToDevice {
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
	pub fn new(bits: FisRegH2DBits, command: u8, control: u8, lba: u64, count: u16, device: u8) -> Self {
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
pub struct FisData {
	fis_type: FisType,
	_bits: u8,
	_rserved0: [u8; 2],
	data: (), // TODO figure out what type this should be
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct FisDmaSetup {
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

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FisPioSetup {
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
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct FisSetDeviceBits {
	fis_type: FisType,
	_bits0: u8,
	_bits1: u8,
	error: u8,
	_reserved: [u8; 4],
}
#[derive(Debug, Copy, Clone)]
#[repr(C, align(256))]
pub struct RecievedFis {
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
