use super::vector::*;

#[derive(Default, Debug, Clone, Copy)]
pub struct Ray {
	pub origin: Point3,
	pub direction: Vec3,
}

impl Ray {
	pub fn at(&self, t: f64) -> Point3 {
		self.origin + (self.direction * t)
	}
}
