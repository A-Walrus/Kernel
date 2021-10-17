pub const SCREEN_SIZE: usize = 480256;

use super::font::FONT;
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
		self * &Vector::new(Terminal::CHAR_WIDTH, Terminal::CHAR_HEIGHT)
	}
}

type Buffer<'a> = &'a mut [Pixel];

#[derive(Copy, Clone, Debug)]
struct Style {}

impl Style {}

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
			style: Style {},
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
		for character in data.chars() {
			if character.is_ascii_control() {
				match character {
					'\n' => self.new_line(),
					'\t' => self.horizontal_tab(),
					'\r' => self.carriage_return(),
					_ => {
						serial_println!("unmatched control: {:?}", character);
					}
				}
			} else {
				self.write_char(Char {
					character,
					style: Style {},
				});
			}
		}
		self.redraw();
	}

	fn carriage_return(&mut self) {
		self.cursor_pos.x = 0;
	}

	fn horizontal_tab(&mut self) {
		const TAB_SIZE: usize = 8;
		self.move_cursor(TAB_SIZE - (self.cursor_pos.x % TAB_SIZE));
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
		const EMPTY_LINE: [Char; WIDTH] = [Char {
			character: ' ',
			style: Style {},
		}; WIDTH];
		self.chars.copy_within(1.., 0);
		self.chars[HEIGHT - 1] = EMPTY_LINE;
	}
	fn draw_char(&mut self, character: Char, pos: CharPos) {
		const MASK: [u8; 8] = [128, 64, 32, 16, 8, 4, 2, 1];
		let pixel_pos = pos.to_pixel();
		if character.character.is_ascii() {
			let ascii = character.character as usize;
			let char_bitmap = &FONT[ascii];
			for row in 0..16 {
				for col in 0..8 {
					let color = if char_bitmap[row] & MASK[col] != 0 {
						Pixel::new(255, 255, 255)
					} else {
						Pixel::new(0, 0, 0)
					};
					self.screen.put_pixel(color, &pixel_pos + &PixelPos::new(col, row))
				}
			}
		}
	}

	pub fn redraw(&mut self) {
		let mut chars = self.chars;
		for (y, line) in chars.iter().enumerate() {
			for (x, character) in line.iter().enumerate() {
				self.draw_char(*character, Vector::new(x, y))
			}
		}

		self.screen.flush()
	}
}
