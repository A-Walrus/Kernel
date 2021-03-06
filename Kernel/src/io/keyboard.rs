use crate::serial_println;
// use crate::serial_print;
use crate::process;
use lazy_static::lazy_static;
use pc_keyboard::{layouts, DecodedKey, HandleControl, KeyCode, Keyboard, ScancodeSet1};
use spin::Mutex;
use x86_64::{
	instructions::port::{Port, PortGeneric, ReadWriteAccess},
	structures::idt::InterruptStackFrame,
};

const DATA_PORT: u16 = 0x60; // Read/Write
const _STATUS_PORT: u16 = 0x64; // Read
const _COMMAND_PORT: u16 = 0x64; // Write

lazy_static! {
	#[doc(hidden)]
	static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
		Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore));
}

/// setup keyboard
pub fn setup() {
	crate::cpu::interrupts::register_callback(1, keyboard_interrupt);
}

fn keyboard_interrupt(_stack_frame: &InterruptStackFrame) {
	read_input()
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
				DecodedKey::Unicode(character) => {
					for (_, process) in process::MAP.lock().iter_mut() {
						if process.terminal == crate::io::buffer::active_term() && process.blocked_on_input() {
							process.append_input(character)
						}
					}

					// let fg_pid = process::foreground_process();
					// process::MAP
					// .lock()
					// 	.get_mut(&fg_pid)
					// 	.expect("foreground process not in hashmap")
					// 	.append_input(character);
				}
				DecodedKey::RawKey(KeyCode::ArrowLeft) => crate::io::buffer::cycle_terms(1),
				DecodedKey::RawKey(KeyCode::ArrowRight) => crate::io::buffer::cycle_terms(-1),
				DecodedKey::RawKey(KeyCode::F1) => crate::process::remove_current_process(),
				DecodedKey::RawKey(key) => serial_println!("{:?}", key),
			}
		}
	}
}
