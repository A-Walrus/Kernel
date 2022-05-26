use super::interrupts::{PICS, PIC_1_OFFSET};
use crate::{cpu::syscalls::Registers, process, serial_println};
use alloc::collections::VecDeque;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::{
	instructions::{hlt, port::*},
	structures::idt::InterruptStackFrame,
};

const CHANNEL_0: u16 = 0x40; // Read/Write
const CHANNEL_2: u16 = 0x42; // Read/Write
const MODE_COMMAND: u16 = 0x43; // Write
const PORT: u16 = 0x61; // Read/Write

const PIT_BASE_FREQ: u64 = 1193182;

/// Milliseconds per PIT
pub const TIMER_NANOS: u64 = 10_000_000;
const QUANTA: usize = 5;

/// A Note or a rest
pub struct Note {
	/// Duration
	pub duration: Duration,
	/// Frequency. None = rest
	pub pitch: Option<u64>,
	playing: bool,
}

impl Note {
	fn new(pitch: Option<u64>, duration: Duration) -> Self {
		Note {
			duration,
			pitch,
			playing: false,
		}
	}

	fn check_done(&self) -> bool {
		let is_done = self.duration.as_nanos() < (TIMER_NANOS as u128) / 2;
		if is_done {
			stop_sound()
		};
		is_done
	}

	fn start_playing(&mut self) {
		self.playing = true;
		match self.pitch {
			Some(freq) => {
				self.decrease_time();
				start_sound();
				set_freq(freq);
			}
			None => {} // rest
		}
	}

	fn decrease_time(&mut self) {
		self.duration = self
			.duration
			.checked_sub(Duration::from_nanos(TIMER_NANOS))
			.unwrap_or(Duration::ZERO);
	}
}

lazy_static! {
	static ref QUEUE: Mutex<VecDeque<Note>> = Mutex::new(VecDeque::new());
}

/// handle note queue
fn handle_queue() {
	let mut queue = QUEUE.lock();
	if queue.is_empty() {
		return;
	}
	let first = &mut queue[0];
	let is_done = first.check_done();
	if is_done {
		queue.pop_front();
		let next = queue.get_mut(0);
		match next {
			Some(note) => {
				note.start_playing();
			}
			None => {}
		}
	} else if !first.playing {
		first.start_playing()
	} else {
		first.decrease_time();
	}
}

/// setup PIT
pub fn setup_pit() {
	let freq_hz = 1_000_000_000 / TIMER_NANOS;
	let divisor = (PIT_BASE_FREQ / freq_hz) as u16;

	let mut data: PortGeneric<u8, ReadWriteAccess> = Port::new(CHANNEL_0);
	let mut command: PortGeneric<u8, ReadWriteAccess> = Port::new(MODE_COMMAND);
	unsafe {
		command.write(0x36);
		data.write((divisor & 0xff) as u8);
		data.write(((divisor >> 8) & 0xff) as u8);
	}
}

/// Add a note to the queue
pub fn queue_note(note: Note) {
	QUEUE.lock().push_back(note);
}

/// set timer frequency
pub fn set_freq(hz: u64) {
	let divisor = (PIT_BASE_FREQ / hz) as u16;

	let mut data: PortGeneric<u8, ReadWriteAccess> = Port::new(CHANNEL_2);
	let mut command: PortGeneric<u8, ReadWriteAccess> = Port::new(MODE_COMMAND);
	unsafe {
		command.write(0xb6);
		data.write((divisor & 0xff) as u8);
		data.write(((divisor >> 8) & 0xff) as u8);
	}
}

/// start sound
pub fn start_sound() {
	let mut port: PortGeneric<u8, ReadWriteAccess> = Port::new(PORT);
	unsafe {
		let temp = port.read();
		if temp != temp | 3 {
			port.write(temp | 3)
		}
	}
}

/// stop sound
pub fn stop_sound() {
	let mut port: PortGeneric<u8, ReadWriteAccess> = Port::new(PORT);

	unsafe {
		let temp = port.read() & 0xFC;

		port.write(temp);
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
	start_sound();
	set_freq(hz);
	wait(duration);
	stop_sound();
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
pub fn setup_time() {
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

/// count of pits since starting current process
pub static mut PROC_COUNTER: usize = 0;

/// total count of pits
static mut PIT_COUNTER: usize = 0;

/// get total count of PIT interrupts
#[inline(never)]
pub fn get_pit_count() -> usize {
	unsafe { PIT_COUNTER }
}

#[allow(dead_code)] // called from asm
#[no_mangle] // called from asm
extern "C" fn handle_timer_inner(registers_ptr: *mut Registers) -> *const u8 {
	let proc_count;
	unsafe {
		proc_count = PROC_COUNTER;
		PROC_COUNTER += 1;
		PIT_COUNTER += 1;
	};
	handle_queue();
	unsafe {
		PICS.lock().notify_end_of_interrupt(PIC_1_OFFSET + 0);
	}
	let running = unsafe { process::RUNNING };
	if running && proc_count >= QUANTA {
		serial_println!("The clock's run out, time's up, over, blaow");

		let registers: &mut Registers;
		unsafe {
			registers = &mut *registers_ptr;
		}
		let stack_frame_ptr = registers.scratch.rsp as *const InterruptStackFrame;
		let stack_frame = unsafe { &*stack_frame_ptr };
		registers.scratch.rsp = stack_frame.stack_pointer.as_u64();
		let registers = *registers;
		let instruction_pointer = stack_frame.instruction_pointer;
		let rflags = stack_frame.cpu_flags;

		// serial_println!("storing: {:?}", registers);
		crate::process::context_switch(process::State::Timer {
			registers,
			instruction_pointer,
			rflags,
		});
	} else {
		return registers_ptr as *const _;
	}
}

const STACK_SIZE: usize = 4096 * 8;
static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

#[allow(dead_code)] // called from asm
#[no_mangle] // called from asm
extern "C" fn get_timer_stack_addr() -> *const u8 {
	// Switch to kernel stack
	let temp_stack: *const u8 = unsafe { STACK.as_ptr().add(STACK_SIZE) };
	return temp_stack;
}

#[naked]
/// Naked ASM timer handler
pub extern "C" fn handle_timer() {
	unsafe {
		asm!(
			// Push scratch registers
			"
			cli
			push rsp
			push rax
			push rcx
			push rdx
			push rdi
			push rsi
			push r8
			push r9
			push r10
			push r11",
			// Push preserved registers
			"
			push rbx
			push rbp
			push r12
			push r13
			push r14
			push r15
			",
			"mov rbx, rsp",              // C calling convention first variable
			"call get_timer_stack_addr", // Hope rbx doesn't get destroyed...
			"mov rsp, rax",
			"mov rdi, rbx", // C calling convention first variable
			"call handle_timer_inner",
			"mov rsp, rax",
			// Pop preserved registers
			"
			pop r15
			pop r14
			pop r13
			pop r12
			pop rbp
			pop rbx",
			// Pop scratch registers
			"
			pop r11
			pop r10
			pop r9
			pop r8
			pop rsi
			pop rdi
			pop rdx
			pop rcx
			pop rax
			pop rsp",
			"iretq",
			options(noreturn)
		);
	}
}

use crate::SOUND_ENABLE;
/// play shutdown song
pub fn play_shutdown_song() {
	if SOUND_ENABLE {
		let eigth_note = Duration::from_millis(375);
		let quarter_note = eigth_note * 2;

		// shutdown
		wait_for_pit();
		play_note(831, eigth_note);
		play_note(623, eigth_note);
		play_note(415, eigth_note);
		play_note(467, quarter_note);
	}
}

/// play startup song
pub fn play_startup_song() {
	if SOUND_ENABLE {
		let eigth_note = Duration::from_millis(230);
		let quarter_note = eigth_note * 2;
		let half_note = eigth_note * 4;
		let quarter_note_triplet = half_note / 3;
		let eigth_note_triplet = quarter_note / 3;
		let sixteenth_note_triplet = eigth_note / 3;

		// startup
		wait_for_pit();
		queue_note(Note::new(Some(623), eigth_note + sixteenth_note_triplet));
		queue_note(Note::new(Some(312), eigth_note_triplet));
		queue_note(Note::new(Some(467), quarter_note));
		queue_note(Note::new(Some(415), quarter_note + eigth_note_triplet));
		queue_note(Note::new(Some(623), quarter_note_triplet));
		queue_note(Note::new(Some(467), half_note));
	}
}
