pub const SCREEN_SIZE: usize = 480256;

use super::font::FONT;
use bootloader::boot_info::FrameBufferInfo;

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

type PixelPos = (usize, usize);

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
		let index = pos.0 + pos.1 * self.info.stride;
		self.back[index] = color;
	}

	pub fn flush(&mut self) {
		self.front.copy_from_slice(self.back);
	}
}

#[macro_export]
macro_rules! as_pixels {
	($buf:expr) => {
		unsafe { &mut *(($buf as *mut [u8]) as *mut [Pixel; SCREEN_SIZE]) }
	};
}
