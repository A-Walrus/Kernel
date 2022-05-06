#![no_main]
#![no_std]

extern crate alloc;
use standard::{init, println, syscalls};

#[no_mangle]
pub extern "C" fn _start() {
	init();
	for _ in 0..10 {
		println!("GuyOS > ");
	}
	println!("starting another process");
	syscalls::exec("/bin/b");
	println!("started!");
	syscalls::exit(0);
}
