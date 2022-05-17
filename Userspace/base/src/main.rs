#![no_main]
#![no_std]

use standard::syscalls;

#[no_mangle]
pub extern "C" fn _start() {
	// YOUR CODE HERE
	syscalls::exit(0);
}
