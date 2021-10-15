pub const SCREEN_SIZE: usize = 480256;

use super::font::FONT;
use bootloader::boot_info::FrameBufferInfo;
use core::ops;

#[macro_export]
macro_rules! as_pixels {
	($buf:expr) => {
		unsafe { &mut *(($buf as *mut [u8]) as *mut [Pixel; SCREEN_SIZE]) }
	};
}

#[repr(align(4))]
#[derive(Copy, Clone)]
pub struct Pixel {
	pub blue: u8,
	pub green: u8,
	pub red: u8,
}

impl Pixel {
	pub fn new(red: u8, green: u8, blue: u8) -> Self {
		Pixel { blue, green, red }
	}
}

#[derive(Copy, Clone)]
pub struct Vector {
	x: usize,
	y: usize,
}
impl Vector {
	pub fn new(x: usize, y: usize) -> Self {
		Vector { x, y }
	}
}

impl<'a, 'b> ops::Add<&'b Vector> for &'a Vector {
	type Output = Vector;
	fn add(self, other: &'b Vector) -> Vector {
		Vector {
			x: self.x + other.x,
			y: self.y + other.y,
		}
	}
}

type PixelPos = Vector;
type CharPos = Vector;

type Buffer<'a> = &'a mut [Pixel];

pub struct Screen<'a> {
	front: Buffer<'a>,
	pub back: Buffer<'a>,
	info: FrameBufferInfo,
}

impl<'a> Screen<'a> {
	pub fn new(front: Buffer<'a>, back: Buffer<'a>, info: FrameBufferInfo) -> Self {
		Screen { front, back, info }
	}

	pub fn put_pixel(&mut self, color: Pixel, pos: PixelPos) {
		self.back[self.pos_to_index(&pos)] = color;
	}

	pub fn flush(&mut self) {
		self.front.copy_from_slice(self.back);
	}

	fn pos_to_index(&self, pos: &PixelPos) -> usize {
		pos.x + pos.y * self.info.stride
	}
}

pub struct Terminal<'a> {
	screen: Screen<'a>,
	cols: usize,
	rows: usize,
	cursor_pos: CharPos,
	//	character array: how is it dynamic size without allocator?
}

impl<'a> Terminal<'a> {
	const CHAR_HEIGHT: usize = 16;
	const CHAR_WIDTH: usize = 8;
	pub fn new(screen: Screen<'a>) -> Self {
		Self {
			screen,
			cols: screen.info.horizontal_resolution / Self::CHAR_WIDTH,
			rows: screen.info.vertical_resolution / Self::CHAR_HEIGHT,
			cursor_pos: Vector::new(0, 0),
		}
	}
	pub fn write(&mut self, data: &str) {}
}
