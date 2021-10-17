pub const SCREEN_SIZE: usize = 480256;

use super::{
	ansi::{Colour, Style},
	font::FONT,
};
use crate::serial_println;
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

#[derive(Copy, Clone, Debug)]
pub struct Vector {
	x: usize,
	y: usize,
}
impl Vector {
	pub fn new(x: usize, y: usize) -> Self {
		Vector { x, y }
	}
}

// vector addition
impl<'a, 'b> ops::Add<&'b Vector> for &'a Vector {
	type Output = Vector;
	fn add(self, other: &'b Vector) -> Vector {
		Vector {
			x: self.x + other.x,
			y: self.y + other.y,
		}
	}
}

// vector multiplication
impl<'a, 'b> ops::Mul<&'b Vector> for &'a Vector {
	type Output = Vector;
	fn mul(self, other: &'b Vector) -> Vector {
		Vector {
			x: self.x * other.x,
			y: self.y * other.y,
		}
	}
}

// vector division
impl<'a, 'b> ops::Div<&'b Vector> for &'a Vector {
	type Output = Vector;
	fn div(self, other: &'b Vector) -> Vector {
		Vector {
			x: self.x / other.x,
			y: self.y / other.y,
		}
	}
}

// scalar multiplication
impl<'a, 'b> ops::Mul<usize> for &'a Vector {
	type Output = Vector;
	fn mul(self, scalar: usize) -> Vector {
		Vector {
			x: self.x * scalar,
			y: self.y * scalar,
		}
	}
}

// scalar division
impl<'a, 'b> ops::Div<usize> for &'a Vector {
	type Output = Vector;
	fn div(self, scalar: usize) -> Vector {
		self * (1 / scalar)
	}
}

type PixelPos = Vector;
type CharPos = Vector;

impl CharPos {
	fn to_pixel(&self) -> PixelPos {
		self / &Vector::new(Terminal::CHAR_WIDTH, Terminal::CHAR_HEIGHT)
	}
}

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

const WIDTH: usize = 80;
const HEIGHT: usize = 25;

#[derive(Copy, Clone, Debug)]
struct Char {
	character: char,
	style: Style,
}

impl Char {
	fn new(character: char) -> Self {
		Self {
			character,
			style: Style::default(),
		}
	}
}

pub struct Terminal<'a> {
	screen: Screen<'a>,
	cursor_pos: CharPos,
	chars: [[Char; WIDTH]; HEIGHT],
}

impl<'a> Terminal<'a> {
	const CHAR_HEIGHT: usize = 16;
	const CHAR_WIDTH: usize = 8;

	pub fn new(screen: Screen<'a>) -> Self {
		Self {
			screen,
			cursor_pos: Vector::new(0, 0),
			chars: [[Char::new(' '); WIDTH]; HEIGHT],
		}
	}
	pub fn write(&mut self, data: &str) {
		// serial print for debugging
		serial_println!("{}", data);

		for character in data.chars() {
			self.write_char(Char {
				character,
				style: Style::default(),
			});
		}
	}

	fn get_char(&mut self, pos: CharPos) -> &mut Char {
		&mut self.chars[pos.y][pos.x]
	}

	fn write_char(&mut self, character: Char) {
		let current = self.get_char(self.cursor_pos);
		*current = character;
		self.move_cursor(1);
	}

	fn move_cursor(&mut self, dist: usize) {
		self.cursor_pos.x = (self.cursor_pos.x + dist) % WIDTH;
		let new_lines = (self.cursor_pos.x + dist) / WIDTH;
		for _ in 0..new_lines {
			self.new_line();
		}
	}

	fn new_line(&mut self) {
		self.cursor_pos.x = 0;
		if self.cursor_pos.y < HEIGHT - 1 {
			self.cursor_pos.y += 1;
		} else {
			self.line_up()
		}
	}

	fn line_up(&mut self) {
		self.chars.copy_within(1.., 0);
		self.redraw();
	}

	fn draw_char(&mut self, character: Char, pos: CharPos) {
		let pixel_pos = pos.to_pixel();
	}

	pub fn redraw(&mut self) {
		// TODO draw

		self.screen.flush()
	}
}
