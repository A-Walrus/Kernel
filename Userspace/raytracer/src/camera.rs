use super::{angles::*, *};
pub struct Camera {
	origin: Point3,
	lower_left_corner: Point3,
	horizontal: Vec3,
	vertical: Vec3,
	u: Vec3,
	v: Vec3,
	w: Vec3,
	lens_radius: f64,
}

use libm::tan;

impl Camera {
	pub fn new(
		look_from: Point3,
		look_at: Point3,
		vup: Vec3,
		vertical_fov_deg: Degrees,
		aspect_ratio: f64,
		aperture: f64,
		focus_dist: f64,
	) -> Camera {
		let theta: Radians = vertical_fov_deg.into();
		let h = tan(theta.0 / 2.0);
		let viewport_height: f64 = 2.0 * h;
		let viewport_width: f64 = aspect_ratio * viewport_height;

		let focal_length: f64 = 1.0;

		let w = (look_from - look_at).normalized();
		let u = Vec3::cross(vup, w);
		let v = Vec3::cross(w, u);

		let origin: Point3 = look_from;
		let horizontal: Vec3 = u * viewport_width * focus_dist;
		let vertical: Vec3 = v * viewport_height * focus_dist;
		let lower_left_corner: Vec3 = origin - horizontal / 2.0 - vertical / 2.0 - w * focus_dist;

		let lens_radius = aperture / 2.0;
		Camera {
			origin,
			horizontal,
			vertical,
			lower_left_corner,
			u,
			v,
			w,
			lens_radius,
		}
	}
	pub fn get_ray(&self, s: f64, t: f64) -> Ray {
		let rd = Vec3::random_in_unit_disk() * self.lens_radius;
		let offset = self.u * rd.x() + self.v * rd.y();
		Ray {
			origin: self.origin + offset,
			direction: self.lower_left_corner + self.horizontal * s + self.vertical * t - self.origin - offset,
		}
	}
}
