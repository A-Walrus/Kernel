#![no_main]
#![no_std]

use standard::syscalls;

#[no_mangle]
pub extern "C" fn _start() {
	for _ in 0..10 {
		syscalls::print("GuyOS >  \n");
	}
	syscalls::exec("/bin/b");
	syscalls::exit(0);
}
