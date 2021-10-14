pub const SCREEN_SIZE: usize = 480256;

use super::font::FONT;
use bootloader::boot_info::FrameBufferInfo;
use core::ops;

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
pub struct PixelPos {
	x: usize,
	y: usize,
}
impl PixelPos {
	pub fn new(x: usize, y: usize) -> Self {
		PixelPos { x, y }
	}
}

impl<'a, 'b> ops::Add<&'b PixelPos> for &'a PixelPos {
	type Output = PixelPos;
	fn add(self, other: &'b PixelPos) -> PixelPos {
		PixelPos {
			x: self.x + other.x,
			y: self.y + other.y,
		}
	}
}

type Buffer<'a> = &'a mut [Pixel];

pub struct Screen<'a> {
	pub front: Buffer<'a>,
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

#[macro_export]
macro_rules! as_pixels {
	($buf:expr) => {
		unsafe { &mut *(($buf as *mut [u8]) as *mut [Pixel; SCREEN_SIZE]) }
	};
}

pub struct TextBuffer<'a> {
	pub screen: Screen<'a>,
}

impl<'a> TextBuffer<'a> {
	const MASK: [u8; 8] = [128, 64, 32, 16, 8, 4, 2, 1];
	pub fn draw_char(&mut self, ascii: usize, mut pos: PixelPos, color: Pixel) {
		let char_bitmap = &FONT[ascii];
		for row in 0..16 {
			for col in 0..8 {
				if char_bitmap[row] & Self::MASK[col] != 0 {
					self.screen.put_pixel(color, pos)
				}
				pos.x += 1;
			}
			pos.y += 1;
			pos.x -= 8;
		}
	}
}
