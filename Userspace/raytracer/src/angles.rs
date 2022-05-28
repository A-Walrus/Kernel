use super::utility::*;
pub struct Degrees(pub f64);
pub struct Radians(pub f64);

impl From<Radians> for Degrees {
	fn from(r: Radians) -> Self {
		Degrees(r.0 * 180.0 / PI)
	}
}

impl From<Degrees> for Radians {
	fn from(d: Degrees) -> Self {
		Radians(d.0 * PI / 180.0)
	}
}
