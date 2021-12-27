use super::font::FONT;
use crate::serial_println;
use bootloader::boot_info::{FrameBuffer, FrameBufferInfo};
use core::{ops, slice};

use alloc::{boxed::Box, vec::Vec};

/// Calculate the length of the framebuffer according to [FrameBufferInfo]. This value may be
/// different from the [FrameBufferInfo::byte_len]
pub fn calc_real_length(framebuffer: &FrameBuffer) -> usize {
	let info = framebuffer.info();
	info.bytes_per_pixel * info.stride * info.vertical_resolution
}

/// Pixel as represented in a framebuffer. Colors in a pixel are ordered BGR, with pixels being
/// aligned to 4 bytes.
#[repr(align(4))]
#[derive(Debug, Copy, Clone)]
pub struct Pixel {
	/// The amount of blue in this color.
	pub blue: u8,
	/// The amount of green in this color.
	pub green: u8,
	/// The amount of red in this color.
	pub red: u8,
}

impl Pixel {
	/// Create a new pixel of a given color.
	pub fn new(red: u8, green: u8, blue: u8) -> Self {
		Pixel { blue, green, red }
	}
}

/// 2D vector/point, used to represent pixel, or character locations.
#[derive(Copy, Clone, Debug)]
pub struct Vector {
	x: usize,
	y: usize,
}

impl Vector {
	/// Create a new vector from x and y coords.
	pub fn new(x: usize, y: usize) -> Self {
		Vector { x, y }
	}
}

/// Scalar addition.
impl<'a, 'b> ops::Add<&'b Vector> for &'a Vector {
	type Output = Vector;
	fn add(self, other: &'b Vector) -> Vector {
		Vector {
			x: self.x + other.x,
			y: self.y + other.y,
		}
	}
}

impl<'a, 'b> ops::Mul<&'b Vector> for &'a Vector {
	type Output = Vector;
	fn mul(self, other: &'b Vector) -> Vector {
		Vector {
			x: self.x * other.x,
			y: self.y * other.y,
		}
	}
}

impl<'a, 'b> ops::Div<&'b Vector> for &'a Vector {
	type Output = Vector;
	fn div(self, other: &'b Vector) -> Vector {
		Vector {
			x: self.x / other.x,
			y: self.y / other.y,
		}
	}
}

/// Scalar multiplication.
impl<'a, 'b> ops::Mul<usize> for &'a Vector {
	type Output = Vector;
	fn mul(self, scalar: usize) -> Vector {
		Vector {
			x: self.x * scalar,
			y: self.y * scalar,
		}
	}
}

/// Scalar division.
impl<'a, 'b> ops::Div<usize> for &'a Vector {
	type Output = Vector;
	fn div(self, scalar: usize) -> Vector {
		self * (1 / scalar)
	}
}

/// Type alias for a [Vector] when used to represent position of a pixel.
pub type PixelPos = Vector;

/// Type alias for a reference to a framebuffer: an array of [Pixel]s.
type Buffer<'a> = &'a mut [Pixel];

/// Styling of a character: color, bold, italic, strikthrough...
/// This is currently not used.
#[derive(Copy, Clone, Debug)]
struct Style {}

impl Style {}

/// A screen has two buffers, front, and back, to be used for double buffering.
/// The front buffer is the one that is mapped to the screen (or a region of the screen), and the
/// back buffer is the one that is written into. The back buffer can be "flushed" onto the front
/// buffer with the [Screen::flush] method.
pub struct Screen<'a> {
	/// The front buffer, visible buffer.
	front: Buffer<'a>,
	/// The back buffer, that is directly written to.
	pub back: Vec<Pixel>,
	/// Some information about the screen: (resolution...).
	info: FrameBufferInfo,
}

impl<'a> Screen<'a> {
	/// New screen from bootloader [FrameBuffer].
	pub fn new_from_framebuffer(framebuffer: &mut FrameBuffer) -> Self {
		let info = framebuffer.info();
		let buffer = framebuffer.buffer_mut();
		let front: &mut [Pixel];
		unsafe {
			front = slice::from_raw_parts_mut(buffer.as_mut_ptr() as *mut Pixel, calc_real_length(framebuffer) / 4);
		}

		Screen::new(front, info)
	}

	/// Create a new screen.
	pub fn new(front: Buffer<'a>, info: FrameBufferInfo) -> Self {
		let vec = vec![Pixel::new(0, 0, 0); front.len()];
		let mut screen = Screen { front, back: vec, info };
		screen.flush();
		screen
	}

	/// Draw a pixel onto the [Screen::back] buffer
	pub fn put_pixel(&mut self, color: Pixel, pos: &PixelPos) {
		let index = self.pos_to_index(pos);
		self.back[index] = color;
	}

	/// Flush back buffer onto front buffer. This is done using a memcpy.
	pub fn flush(&mut self) {
		self.front.copy_from_slice(&self.back);
	}

	/// Convert 2D [PixelPos] into 1D index into [Buffer]. This is calculated using
	/// [FrameBufferInfo::stride] and not the x resolution, since there could be padding outside of
	/// the screen.
	fn pos_to_index(&self, pos: &PixelPos) -> usize {
		pos.x + pos.y * self.info.stride
	}
}

/// A utf-8 character and it's style. The [Terminal] is built as a grid of these.
#[derive(Copy, Clone, Debug)]
struct Char {
	character: char,
	style: Style,
}

impl Char {
	/// Create a new [Char] from a [char], using a default empty style.
	fn new(character: char) -> Self {
		Self {
			character,
			style: Style {},
		}
	}
}

/// A terminal with a screen, a cursor, and a grid of [Char]s.
pub struct Terminal<'a> {
	/// The screen that this terminal controls.
	screen: Screen<'a>,
	/// The current position of the cursor.
	cursor_pos: usize,
	/// Grid of characters with their styles, representing what's currently on screen.
	chars: Vec<Char>,
	/// Width of screen in characters.
	width: usize,
	/// Height of screen in characters.
	height: usize,
	/// Pixels of empty row for efficient clearing.
	empty: Vec<Pixel>,
}

impl<'a> Terminal<'a> {
	/// The height of a single character in pixels.
	const CHAR_HEIGHT: usize = 16;
	/// The width of a single character in pixels.
	const CHAR_WIDTH: usize = 8;

	/// Create a new [Terminal] from a [Screen]. This takes ownership of the screen, as the
	/// terminal is now the one responsible for it.
	pub fn new(screen: Screen<'a>) -> Self {
		let width = screen.info.horizontal_resolution / Terminal::CHAR_WIDTH;
		let height = screen.info.vertical_resolution / Terminal::CHAR_HEIGHT;
		Self {
			screen,
			width,
			height,
			cursor_pos: 0,
			chars: vec![Char::new(' '); width * height],
			empty: vec![Pixel::new(0, 0, 0); width * Terminal::CHAR_HEIGHT * Terminal::CHAR_WIDTH],
		}
	}

	fn pixels_per_char_row(&self) -> usize {
		self.width * Terminal::CHAR_HEIGHT * Terminal::CHAR_WIDTH
	}

	/// write a string to the terimnal.
	pub fn write(&mut self, data: &str) {
		let start_line = self.cursor_pos / self.width;
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
		let end_line = self.cursor_pos / self.width;
		let pixels_per_char_row = self.pixels_per_char_row();
		let start = start_line * pixels_per_char_row;
		let end = (end_line + 1) * pixels_per_char_row;
		self.screen.front[start..end].copy_from_slice(&self.screen.back[start..end]);
	}

	/// Move cursor to beginning of line.
	fn carriage_return(&mut self) {
		self.cursor_pos = self.cursor_pos % self.width;
	}

	const TAB_SIZE: usize = 8;
	/// Tab horizontally: move cursor forword to nearest multiple of [Terminal::TAB_SIZE].
	fn horizontal_tab(&mut self) {}

	/// Write a single [Char] onto the screen, at [Terminal::cursor_pos].
	fn write_char(&mut self, character: Char) {
		self.draw_char(character, self.cursor_pos);
		self.chars[self.cursor_pos] = character;
		self.move_cursor(1);
	}

	/// Add a new line.
	/// * At bottom of screen: Move entire screen up with [Terminal::line_up].
	/// * Otherwise: Move cusor to start of next line.
	fn new_line(&mut self) {
		self.cursor_pos -= self.cursor_pos % self.width;
		if self.cursor_pos / self.width < self.height - 1 {
			self.cursor_pos += self.width;
		} else {
			self.line_up()
		}
	}

	/// Move the cursor a certain amount forward. Wraps to next line if you reach the end.
	fn move_cursor(&mut self, dist: usize) {
		let old_x = self.cursor_pos % self.width;
		let new_x = (old_x + dist) % self.width;
		self.cursor_pos = self.cursor_pos - old_x + new_x;

		let new_lines = (old_x + dist) / self.width;
		for _ in 0..new_lines {
			self.new_line();
		}
	}

	/// Move entire screen up a line. This involves a memcpy of the characters array
	fn line_up(&mut self) {
		self.chars.copy_within(self.width.., 0);
		let end = self.width * self.height;
		self.chars[end - self.width..end].fill(Char::new(' '));

		let pixels_per_char_row = self.pixels_per_char_row();

		self.screen.back.copy_within(pixels_per_char_row.., 0);

		let len = self.screen.back.len();
		self.screen.back[len - pixels_per_char_row..].clone_from_slice(&self.empty);

		self.screen.flush();
	}

	fn index_to_pixel(&self, index: usize) -> PixelPos {
		let x = index % self.width;
		let y = index / self.width;
		PixelPos::new(x * Terminal::CHAR_WIDTH, y * Terminal::CHAR_HEIGHT)
	}

	/// Draw a character at a certain position. This writes pixels to the back buffer. In order to
	/// see changes on screen you must flush the screen.
	fn draw_char(&mut self, character: Char, pos: usize) {
		const MASK: [u8; 8] = [128, 64, 32, 16, 8, 4, 2, 1];
		let mut pos = self.index_to_pixel(pos);
		if character.character.is_ascii() {
			let ascii = character.character as usize;
			let char_bitmap = &FONT[ascii];
			for row in 0..Terminal::CHAR_HEIGHT {
				for col in 0..Terminal::CHAR_WIDTH {
					let color = if char_bitmap[row] & MASK[col] != 0 {
						Pixel::new(255, 255, 255)
					} else {
						Pixel::new(0, 0, 0)
					};
					self.screen.put_pixel(color, &pos);
					pos.x += 1;
				}
				pos.x -= Terminal::CHAR_WIDTH;
				pos.y += 1;
			}
		}
	}
}
