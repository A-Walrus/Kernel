use crate::utility::{random_double, random_in_range};

use super::{material::Material, ray::*, vector::*};

use alloc::{boxed::Box, vec::Vec};

#[derive(Default, Copy, Clone)]
pub struct HitRecord<'a> {
	pub point: Point3,
	pub normal: Vec3,
	pub t: f64,
	pub front_face: bool,
	pub material: &'a dyn Material,
}

use libm::sqrt;

impl<'a> HitRecord<'a> {
	fn set_face_normal(&mut self, ray: &Ray, outward_normal: &Vec3) {
		self.front_face = Vec3::dot(ray.direction, *outward_normal) < 0.0;
		self.normal = if self.front_face {
			*outward_normal
		} else {
			-*outward_normal
		}
	}
}

pub trait Hittable {
	fn hit<'a>(&'a self, ray: &Ray, t_min: f64, t_max: f64, record: &mut HitRecord<'a>) -> bool;
}

pub struct Sphere {
	pub center: Point3,
	pub radius: f64,
	pub material: Box<dyn Material>,
}

impl Hittable for Sphere {
	fn hit<'a>(&'a self, ray: &Ray, t_min: f64, t_max: f64, record: &mut HitRecord<'a>) -> bool {
		let oc = ray.origin - self.center;
		let a = ray.direction.length_squared();
		let half_b = Vec3::dot(oc, ray.direction);
		let c = oc.length_squared() - (self.radius * self.radius);
		let discriminant = (half_b * half_b) - (a * c);
		if discriminant < 0.0 {
			return false;
		} else {
			let sqrt_d = sqrt(discriminant);
			let mut root = (-half_b - sqrt_d) / a;
			if root < t_min || t_max < root {
				root = (-half_b + sqrt_d) / a;
				if root < t_min || t_max < root {
					return false;
				}
			}
			record.t = root;
			record.point = ray.at(root);
			let outward_normal = (record.point - self.center) / self.radius;
			record.set_face_normal(ray, &outward_normal);
			record.material = self.material.as_ref();
			true
		}
	}
}

pub struct HittableList {
	objects: Vec<Box<dyn Hittable>>,
}

impl HittableList {
	pub fn new() -> Self {
		HittableList { objects: Vec::new() }
	}

	pub fn add(&mut self, object: Box<dyn Hittable>) {
		self.objects.push(object);
	}

	pub fn random() -> Box<dyn Hittable> {
		use super::material::*;

		let mut world = HittableList::new();

		let material_ground = Box::new(Lambertian {
			albedo: Color::new(0.5, 0.5, 0.5),
		});

		world.add(Box::new(Sphere {
			center: Point3::new(0.0, -1000.0, 0.0),
			radius: 1000.0,
			material: material_ground,
		}));

		for a in -11..11 {
			for b in -11..11 {
				let choose_mat = random_double();
				let center: Point3 =
					Point3::new(a as f64 + 0.9 * random_double(), 0.2, b as f64 + 0.9 * random_double());

				if (center - Point3::new(4.0, 0.2, 0.0)).length() > 0.9 {
					if choose_mat < 0.8 {
						// Diffuse
						let albedo = Color::random() * Color::random();
						let material = Box::new(Lambertian { albedo });
						world.add(Box::new(Sphere {
							center,
							radius: 0.2,
							material,
						}));
					} else if choose_mat < 0.95 {
						// Metal
						let albedo = Color::random_in_range(0.5, 1.0);
						let fuzz = random_in_range(0.0, 0.5);
						let material = Box::new(Metal { albedo, fuzz });
						world.add(Box::new(Sphere {
							center,
							radius: 0.2,
							material,
						}));
					} else {
						// Glass
						let material = Box::new(Dielectric { refraction_index: 1.5 });
						world.add(Box::new(Sphere {
							center,
							radius: 0.2,
							material,
						}));
					}
				}
			}
		}
		let material_1 = Box::new(Dielectric { refraction_index: 1.5 });
		world.add(Box::new(Sphere {
			center: Point3::new(0.0, 1.0, 0.0),
			radius: 1.0,
			material: material_1,
		}));

		let material_2 = Box::new(Lambertian {
			albedo: Color::new(0.4, 0.2, 0.1),
		});
		world.add(Box::new(Sphere {
			center: Point3::new(-4.0, 1.0, 0.0),
			radius: 1.0,
			material: material_2,
		}));

		let material_3 = Box::new(Metal {
			albedo: Color::new(0.7, 0.6, 0.5),
			fuzz: 0.0,
		});
		world.add(Box::new(Sphere {
			center: Point3::new(4.0, 1.0, 0.0),
			radius: 1.0,
			material: material_3,
		}));

		Box::new(world)
	}
}

impl Hittable for HittableList {
	fn hit<'a>(&'a self, ray: &Ray, t_min: f64, t_max: f64, record: &mut HitRecord<'a>) -> bool {
		let mut temp = HitRecord::default();
		let mut hit_anything = false;
		let mut closest_so_far = t_max;

		for object in &self.objects {
			if object.hit(ray, t_min, closest_so_far, &mut temp) {
				hit_anything = true;
				closest_so_far = temp.t;
				*record = temp;
			}
		}
		return hit_anything;
	}
}
