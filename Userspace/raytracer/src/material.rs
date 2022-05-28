use crate::{
	hittable::HitRecord,
	ray::Ray,
	utility::random_double,
	vector::{Color, Vec3},
};
use libm::{fabs, fmin, pow, sqrt};

pub trait Material {
	fn scatter(&self, ray_in: &Ray, rec: &HitRecord) -> Option<(Color, Ray)>;
}

const DEFAULT: BlackHole = BlackHole {};
impl Default for &dyn Material {
	fn default() -> Self {
		&DEFAULT
	}
}

struct BlackHole {}

impl Material for BlackHole {
	fn scatter(&self, _ray_in: &Ray, _rec: &HitRecord) -> Option<(Color, Ray)> {
		None
	}
}

pub struct Lambertian {
	pub albedo: Color,
}

impl Material for Lambertian {
	fn scatter(&self, _ray_in: &Ray, rec: &HitRecord) -> Option<(Color, Ray)> {
		let scatter_dir = Vec3::random_in_hemisphere(&rec.normal);
		let child_ray = Ray {
			origin: rec.point,
			direction: scatter_dir,
		};
		return Some((self.albedo, child_ray));
	}
}

pub struct Metal {
	pub albedo: Color,
	pub fuzz: f64,
}

impl Material for Metal {
	fn scatter(&self, ray_in: &Ray, rec: &HitRecord) -> Option<(Color, Ray)> {
		let reflected = Vec3::reflect(&ray_in.direction, &rec.normal);
		if Vec3::dot(reflected, rec.normal) > 0.0 {
			Some((
				self.albedo,
				Ray {
					origin: rec.point,
					direction: reflected + Vec3::random_in_unit_sphere() * self.fuzz,
				},
			))
		} else {
			None
		}
	}
}

fn refract(uv: &Vec3, n: &Vec3, etai_over_etat: f64) -> Vec3 {
	let cos_theta = fmin(Vec3::dot(-*uv, *n), 1.0);
	let ray_out_perp = (*uv + *n * cos_theta) * etai_over_etat;
	let ray_out_parallel = *n * -sqrt(fabs(1.0 - ray_out_perp.length_squared()));
	ray_out_perp + ray_out_parallel
}

// Schlick approximation for reflectance
fn reflectance(cosine: f64, ref_idx: f64) -> f64 {
	let mut r0 = (1.0 - ref_idx) / (1.0 + ref_idx);
	r0 = r0 * r0;
	r0 + (1.0 - r0) * pow(1.0 - cosine, 5.)
}

pub struct Dielectric {
	pub refraction_index: f64,
}

impl Material for Dielectric {
	fn scatter(&self, ray_in: &Ray, rec: &HitRecord) -> Option<(Color, Ray)> {
		let refraction_ratio = if rec.front_face {
			1.0 / self.refraction_index
		} else {
			self.refraction_index
		};
		let unit_direction = ray_in.direction.normalized();

		let cos_theta = fmin(Vec3::dot(-unit_direction, rec.normal), 1.0);
		let sin_theta = sqrt(1.0 - cos_theta * cos_theta);

		let cannot_refract = refraction_ratio * sin_theta > 1.0;
		let direction = if cannot_refract || reflectance(cos_theta, refraction_ratio) > random_double() {
			Vec3::reflect(&unit_direction, &rec.normal)
		} else {
			refract(&unit_direction, &rec.normal, refraction_ratio)
		};

		let ray = Ray {
			origin: rec.point,
			direction,
		};
		Some((Color::new(1.0, 1.0, 1.0), ray))
	}
}
