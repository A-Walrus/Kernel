use lazy_static::lazy_static;
use nanorand::{Rng, WyRand};
use spin::Mutex;
use standard::println;

lazy_static! {
	static ref RNG: Mutex<WyRand> = Mutex::new(WyRand::new_seed(0x5502cf95915b7ef9));
}
pub const INFINITY: f64 = f64::INFINITY;
pub const PI: f64 = core::f64::consts::PI;

pub fn clamp(x: f64, min: f64, max: f64) -> f64 {
	if x < min {
		min
	} else if x > max {
		max
	} else {
		x
	}
}

pub fn random_double() -> f64 {
	RNG.lock().generate()
}

pub fn random_in_range(min: f64, max: f64) -> f64 {
	min + (max - min) * random_double()
}
