use crate::cpu::interrupts::{get_pit_count, TIMER_NANOS};
use x86_64::instructions::hlt;

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

/// Number of ticks
pub struct Ticks(pub u64);

impl Sub for Ticks {
	type Output = Ticks;

	fn sub(self, rhs: Self) -> Self::Output {
		Ticks(self.0 - rhs.0)
	}
}

impl Add for Ticks {
	type Output = Ticks;

	fn add(self, rhs: Self) -> Self::Output {
		Ticks(self.0 + rhs.0)
	}
}

impl Into<Duration> for Ticks {
	fn into(self) -> Duration {
		Duration::from_nanos(self.0 / unsafe { TICKS_PER_NANOSEC })
	}
}

/// Play a note of a certain frequency for a certain duration
pub fn play_note(hz: u64, duration: Duration) {
	crate::cpu::interrupts::start_sound();
	crate::cpu::interrupts::set_freq(hz);
	crate::util::wait(duration);
	crate::cpu::interrupts::stop_sound();
}

use core::ops::*;

use core::time::Duration;

static mut TICKS_PER_NANOSEC: u64 = 0;

/// wait a duration
pub fn wait(duration: Duration) {
	let nanos = duration.as_nanos();
	let times = nanos / TIMER_NANOS as u128;
	for _ in 0..times {
		wait_for_pit()
	}
}

/// wait for timer interrupt
pub fn wait_for_pit() {
	let start_count = get_pit_count();
	while get_pit_count() == start_count {
		hlt();
	}
}

fn measure_ticks_per_pit() -> u64 {
	let n = 4;
	wait_for_pit();
	let start_ticks = get_ticks().0;
	for _ in 0..n {
		wait_for_pit();
	}
	let end_ticks = get_ticks().0;
	(end_ticks - start_ticks) / n
}

/// setup time
pub fn setup() {
	use crate::cpu::interrupts::*;
	let ticks_per_pit = measure_ticks_per_pit();
	unsafe {
		TICKS_PER_NANOSEC = ticks_per_pit / TIMER_NANOS;
	}
}

/// get the current time
pub fn get_ticks() -> Ticks {
	let time_low: u32;
	let time_high: u32;
	unsafe {
		asm!(
		"rdtsc",
		out("edx") time_high,
		out("eax") time_low,
		 );
	}
	Ticks((time_low as u64) | ((time_high as u64) << 32))
}

/// Get the current time
pub fn get_time() -> Duration {
	let ticks = get_ticks();
	ticks.into()
}
