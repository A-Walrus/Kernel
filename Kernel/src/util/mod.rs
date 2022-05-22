/// Module for IO, similair to std::io, which is not available in a no-std project
pub mod io;

/// Module for dealing with QEMU
pub mod qemu {

	// pub enum QemuExitCode {
	// Success = 0x10,
	// Failed = 0x11,
	// }

	/// exit QEMU
	pub fn exit() -> ! {
		use x86_64::instructions::port::Port;
		unsafe {
			let mut port = Port::new(0xf4);
			port.write(0x10 as u32);
		}
		loop {} // this shouldn't be reached, because the last command shutds down computer, but it proves to the compiler I never return
	}
}
