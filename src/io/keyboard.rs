use crate::{serial_print, serial_println};
use lazy_static::lazy_static;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;
use x86_64::instructions::port::{Port, PortGeneric, ReadWriteAccess};

const DATA_PORT: u16 = 0x60; // Read/Write
const _STATUS_PORT: u16 = 0x64; // Read
const _COMMAND_PORT: u16 = 0x64; // Write

lazy_static! {
	#[doc(hidden)]
	static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
		Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore));
}

/// Called on keyboard interrupt. This reads the scan code from the keyboard data port, and passes
/// it to KEYBOARD for parsing (through [parse_scan_code]).
pub fn read_input() {
	let mut data_port: PortGeneric<u8, ReadWriteAccess> = Port::new(DATA_PORT);
	let scancode;
	unsafe {
		scancode = data_port.read();
	}
	parse_scan_code(scancode);
}

/// Pass scancode to KEYBOARD for parsing. If KEYBOARD has a key event, print it to serial.
fn parse_scan_code(scancode: u8) {
	let mut keyboard = KEYBOARD.lock();
	if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
		if let Some(key) = keyboard.process_keyevent(key_event) {
			match key {
				DecodedKey::Unicode(character) => print!("{}", character),
				DecodedKey::RawKey(key) => serial_println!("{:?}", key),
			}
		}
	}
}
