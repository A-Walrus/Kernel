use crate::{serial_print, serial_println};
use lazy_static::lazy_static;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;
use x86_64::instructions::port::{Port, PortGeneric, ReadOnlyAccess, ReadWriteAccess, WriteOnlyAccess};

const DATA_PORT: u16 = 0x60; // Read/Write
const STATUS_PORT: u16 = 0x64; // Read
const COMMAND_PORT: u16 = 0x64; // Write

lazy_static! {
	static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
		Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore));
}

// called on keyboard interrupt
pub fn read_input() {
	let mut data_port: PortGeneric<u8, ReadWriteAccess> = Port::new(DATA_PORT);
	let scancode;
	unsafe {
		scancode = data_port.read();
	}
	parse_scan_code(scancode);
}

fn parse_scan_code(scancode: u8) {
	let mut keyboard = KEYBOARD.lock();
	if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
		if let Some(key) = keyboard.process_keyevent(key_event) {
			match key {
				DecodedKey::Unicode(character) => serial_print!("{}", character),
				DecodedKey::RawKey(key) => serial_println!("{:?}", key),
			}
		}
	}
}
