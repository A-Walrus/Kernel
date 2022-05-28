use core::ops;

use standard::syscalls::File;

use crate::utility::{clamp, random_double, random_in_range};

use alloc::format;

use standard::io::*;

use libm::sqrt;

#[derive(Default, Debug, Copy, Clone)]
pub struct Vec3(f64, f64, f64);

impl Vec3 {
	pub const fn new(x: f64, y: f64, z: f64) -> Self {
		Vec3 { 0: x, 1: y, 2: z }
	}

	pub fn x(&self) -> f64 {
		self.0
	}

	pub fn y(&self) -> f64 {
		self.1
	}

	pub fn z(&self) -> f64 {
		self.2
	}

	pub fn length(&self) -> f64 {
		sqrt(self.length_squared())
	}

	pub fn length_squared(&self) -> f64 {
		self.0 * self.0 + self.1 * self.1 + self.2 * self.2
	}

	pub fn normalize(&mut self) {
		*self /= self.length()
	}

	pub fn normalized(&self) -> Vec3 {
		*self / self.length()
	}

	pub fn dot(a: Vec3, b: Vec3) -> f64 {
		a.0 * b.0 + a.1 * b.1 + a.2 * b.2
	}

	pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
		Vec3 {
			0: a.1 * b.2 - a.2 * b.1,
			1: a.2 * b.0 - a.0 * b.2,
			2: a.0 * b.1 - a.1 * b.0,
		}
	}

	pub fn random() -> Vec3 {
		Vec3(random_double(), random_double(), random_double())
	}

	pub fn random_in_range(min: f64, max: f64) -> Vec3 {
		Vec3(
			random_in_range(min, max),
			random_in_range(min, max),
			random_in_range(min, max),
		)
	}

	pub fn random_in_unit_sphere() -> Vec3 {
		loop {
			let rand = Vec3::random_in_range(-1.0, 1.0);
			if rand.length() >= 1.0 {
				continue;
			}
			return rand;
		}
	}

	pub fn random_unit_vector() -> Vec3 {
		Vec3::random_in_unit_sphere().normalized()
	}

	pub fn random_in_hemisphere(normal: &Vec3) -> Vec3 {
		let in_unit_sphere = Vec3::random_in_unit_sphere();
		if Vec3::dot(in_unit_sphere, *normal) > 0.0 {
			in_unit_sphere
		} else {
			-in_unit_sphere
		}
	}

	pub fn random_in_unit_disk() -> Vec3 {
		loop {
			let p = Vec3(random_in_range(-1.0, 1.0), random_in_range(-1.0, 1.0), 0.0);
			if p.length_squared() >= 1.0 {
				continue;
			}
			return p;
		}
	}

	pub fn reflect(v: &Vec3, n: &Vec3) -> Vec3 {
		*v - *n * 2.0 * Vec3::dot(*v, *n)
	}
}

impl ops::AddAssign for Vec3 {
	fn add_assign(&mut self, rhs: Self) {
		self.0 += rhs.0;
		self.1 += rhs.1;
		self.2 += rhs.2;
	}
}

impl ops::MulAssign<f64> for Vec3 {
	fn mul_assign(&mut self, rhs: f64) {
		self.0 *= rhs;
		self.1 *= rhs;
		self.2 *= rhs;
	}
}

impl ops::DivAssign<f64> for Vec3 {
	fn div_assign(&mut self, rhs: f64) {
		*self *= 1.0 / rhs;
	}
}

impl ops::Add for Vec3 {
	type Output = Vec3;
	fn add(self, rhs: Self) -> Self::Output {
		Vec3::new(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2)
	}
}

impl ops::Sub for Vec3 {
	type Output = Vec3;
	fn sub(self, rhs: Self) -> Self::Output {
		Vec3::new(self.0 - rhs.0, self.1 - rhs.1, self.2 - rhs.2)
	}
}

impl ops::Mul<f64> for Vec3 {
	type Output = Vec3;
	fn mul(self, rhs: f64) -> Self::Output {
		Vec3::new(self.0 * rhs, self.1 * rhs, self.2 * rhs)
	}
}

impl ops::Mul<Vec3> for Vec3 {
	type Output = Vec3;
	fn mul(self, rhs: Vec3) -> Self::Output {
		Vec3::new(self.0 * rhs.0, self.1 * rhs.1, self.2 * rhs.2)
	}
}

impl ops::Div<f64> for Vec3 {
	type Output = Vec3;
	fn div(self, rhs: f64) -> Self::Output {
		self * (1.0 / rhs)
	}
}

impl ops::Neg for Vec3 {
	type Output = Vec3;

	fn neg(self) -> Self::Output {
		self * -1.0
	}
}

pub type Point3 = Vec3;
pub type Color = Vec3;

impl Color {
	pub fn write(&self, samples_per_pixel: usize, file: &mut File) {
		let mut r = self.x();
		let mut g = self.y();
		let mut b = self.z();

		let scale = 1.0 / samples_per_pixel as f64;
		r = sqrt(r * scale);
		g = sqrt(g * scale);
		b = sqrt(b * scale);
		let ir = (256.0 * clamp(r, 0.0, 0.999)) as u8;
		let ig = (256.0 * clamp(g, 0.0, 0.999)) as u8;
		let ib = (256.0 * clamp(b, 0.0, 0.999)) as u8;

		file.write(format!("{} {} {}\n", ir, ig, ib).as_bytes())
			.expect("Failed to write to file");
	}
}
