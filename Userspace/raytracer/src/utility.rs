use lazy_static::lazy_static;
use rand::{prelude::SmallRng, Rng, SeedableRng};
use spin::Mutex;

lazy_static! {
	static ref RNG: Mutex<SmallRng> = Mutex::new(SmallRng::seed_from_u64(0));
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
	0.5
}

pub fn random_in_range(min: f64, max: f64) -> f64 {
	min + (max - min) * random_double()
}
