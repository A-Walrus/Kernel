#![no_main]
#![no_std]

mod angles;
mod camera;
mod hittable;
mod material;
mod ray;
mod utility;
mod vector;
use camera::Camera;
use hittable::{HitRecord, Hittable, HittableList};
use material::*;
use ray::*;
use utility::*;
use vector::*;

use standard::{
	io::{Read, Write},
	syscalls::*,
	*,
};
extern crate alloc;
use alloc::{boxed::Box, format, vec::Vec};

extern crate lazy_static;

use crate::{angles::Degrees, hittable::Sphere};

fn ray_color(ray: &Ray, world: &Box<dyn Hittable>, depth: usize) -> Color {
	let mut rec = HitRecord::default();
	if depth <= 0 {
		return Color::new(0.0, 0.0, 0.0);
	}
	if world.hit(ray, 0.001, INFINITY, &mut rec) {
		match rec.material.scatter(ray, &rec) {
			Some((attenuation, ray)) => {
				return ray_color(&ray, world, depth - 1) * attenuation;
			}
			None => {
				return Color::new(0.0, 0.0, 0.0);
			}
		}
	}
	let unit_dir = ray.direction.normalized();
	let t = 0.5 * (unit_dir.y() + 1.0);
	Color::new(1.0, 1.0, 1.0) * (1.0 - t) + Color::new(0.5, 0.7, 1.0) * t
}

#[no_mangle]
pub extern "C" fn main() -> isize {
	let draw = get_args().contains(&"--draw");

	println!("Starting");

	let mut file = File::create("/image.ppm").expect("Failed to create file");

	println!("File created");
	// Image
	const ASPECT_RATIO: f64 = 3.0 / 2.0;
	const IMAGE_WIDTH: usize = 120;
	const IMAGE_HEIGHT: usize = (IMAGE_WIDTH as f64 / ASPECT_RATIO) as usize;
	const SAMPLES_PER_PIXEL: usize = 5;
	const MAX_DEPTH: usize = 50;

	// World

	let world = HittableList::random();

	// Camera

	let look_from = Point3::new(13.0, 2.0, 3.0);
	let look_at = Point3::new(0.0, 0.0, 0.0);
	let vup = Vec3::new(0.0, 1.0, 0.0);
	let dist_to_focus = 10.0;
	let aperture = 0.1;
	let cam = Camera::new(
		look_from,
		look_at,
		vup,
		Degrees(20.0),
		ASPECT_RATIO,
		aperture,
		dist_to_focus,
	);
	// Render

	file.write("P3\n".as_bytes());
	file.write(format!("{} {}\n", IMAGE_WIDTH, IMAGE_HEIGHT).as_bytes());
	file.write("255\n".as_bytes());

	for j in (0..IMAGE_HEIGHT).rev() {
		if !draw {
			println!("Scanlines remaining: {}", j);
		}
		for i in 0..IMAGE_WIDTH {
			let mut pixel_color = Color::new(0.0, 0.0, 0.0);
			for _ in 0..SAMPLES_PER_PIXEL {
				let u = (i as f64 + random_double()) / (IMAGE_WIDTH - 1) as f64;
				let v = (j as f64 + random_double()) / (IMAGE_HEIGHT - 1) as f64;
				let ray = cam.get_ray(u, v);
				pixel_color += ray_color(&ray, &world, MAX_DEPTH);
			}
			pixel_color.write(SAMPLES_PER_PIXEL, &mut file, IMAGE_HEIGHT - j, i, draw);
		}
	}
	println!("Done");
	return 0;
}
