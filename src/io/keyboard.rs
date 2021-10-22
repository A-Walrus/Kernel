use crate::serial_println;
use x86_64::instructions::port::{Port, PortGeneric, ReadOnlyAccess, ReadWriteAccess, WriteOnlyAccess};
const DATA_PORT: u16 = 0x60; // Read/Write
const STATUS_PORT: u16 = 0x64; // Read
const COMMAND_PORT: u16 = 0x64; // Write

pub fn read_key() {
	let mut data_port: PortGeneric<u8, ReadWriteAccess> = Port::new(DATA_PORT);
	let data;
	unsafe {
		data = data_port.read();
	}
	serial_println!("{}", data);
}
