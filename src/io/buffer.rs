use super::{font::FONT, mask_table::MASK_TABLE};
use crate::serial_println;
use bootloader::boot_info::{FrameBuffer, FrameBufferInfo};
use core::{ops, slice};

use alloc::{boxed::Box, vec::Vec};

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
/// Type alias for a [Vector] when used to represent position of a character.
pub type CharPos = Vector;

impl CharPos {
	/// Convert to [PixelPos] of the top left corner of the character. This is dependant on
	/// [Terminal::CHAR_WIDTH], [Terminal::CHAR_HEIGHT], and the offset, if there is any.
	fn to_pixel(&self) -> PixelPos {
		self * &Vector::new(Terminal::CHAR_WIDTH, Terminal::CHAR_HEIGHT)
	}
}

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
/// buffer with the [Terminal::flush] method.
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
			front = slice::from_raw_parts_mut(buffer.as_mut_ptr() as *mut Pixel, buffer.len() / 4);
		}

		Screen::new(front, info)
	}

	/// Create a new screen.
	pub fn new(front: Buffer<'a>, info: FrameBufferInfo) -> Self {
		let mut vec = vec![Pixel::new(0, 0, 0); front.len()];
		Screen { front, back: vec, info }
	}

	/// Draw a pixel onto the [Screen::back] buffer
	pub fn put_pixel(&mut self, color: Pixel, pos: PixelPos) {
		let index = self.pos_to_index(&pos);
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

/// Width of terminal in characters.
const WIDTH: usize = 80;
/// Height of terminal in characters.
const HEIGHT: usize = 25;

/// A utf-8 character and it's style. The terminal is built as a grid of these.
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
	cursor_pos: CharPos,
	/// Grid of characters with their styles, representing what's currently on screen.
	chars: [[Char; WIDTH]; HEIGHT],
}

impl<'a> Terminal<'a> {
	/// The height of a single character in pixels.
	const CHAR_HEIGHT: usize = 16;
	/// The width of a single character in pixels.
	const CHAR_WIDTH: usize = 8;

	/// Create a new [Terminal] from a [Screen]. This takes ownership of the screen, as the
	/// terminal is now the one responsible for it.
	pub fn new(screen: Screen<'a>) -> Self {
		Self {
			screen,
			cursor_pos: Vector::new(0, 0),
			chars: [[Char::new(' '); WIDTH]; HEIGHT],
		}
	}

	/// write a string to the terimnal.
	pub fn write(&mut self, data: &str) {
		serial_println!("started printing");
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
		serial_println!("finished printing");
		self.flush();
		serial_println!("flushed");
	}

	/// Move cursor to beginning of line.
	fn carriage_return(&mut self) {
		self.cursor_pos.x = 0;
	}

	const TAB_SIZE: usize = 8;
	/// Tab horizontally: move cursor forword to nearest multiple of [Terminal::TAB_SIZE].
	fn horizontal_tab(&mut self) {
		self.move_cursor(Terminal::TAB_SIZE - (self.cursor_pos.x % Terminal::TAB_SIZE));
	}

	/// Get a mutable reference to the character at a certain position.
	fn get_char(&mut self, pos: CharPos) -> &mut Char {
		&mut self.chars[pos.y][pos.x]
	}

	/// Write a single [Char] onto the screen, at [Terminal::cursor_pos].
	fn write_char(&mut self, character: Char) {
		self.draw_char(character, self.cursor_pos);

		let current = self.get_char(self.cursor_pos);
		*current = character;
		self.move_cursor(1);
	}

	/// Move the cursor a certain amount forward. Wraps to next line if you reach the end.
	fn move_cursor(&mut self, dist: usize) {
		self.cursor_pos.x = (self.cursor_pos.x + dist) % WIDTH;
		let new_lines = (self.cursor_pos.x + dist) / WIDTH;
		for _ in 0..new_lines {
			self.new_line();
		}
	}

	/// Add a new line.
	/// * At bottom of screen: Move entire screen up with [Terminal::line_up].
	/// * Otherwise: Move cusor to start of next line.
	fn new_line(&mut self) {
		self.cursor_pos.x = 0;
		if self.cursor_pos.y < HEIGHT - 1 {
			self.cursor_pos.y += 1;
		} else {
			self.line_up()
		}
	}

	/// Move entire screen up a line. This involves a memcpy of the characters array, and redrawing
	/// the entire screen.
	fn line_up(&mut self) {
		const EMPTY_LINE: [Char; WIDTH] = [Char {
			character: ' ',
			style: Style {},
		}; WIDTH];
		self.chars.copy_within(1.., 0);
		self.chars[HEIGHT - 1] = EMPTY_LINE;
		self.redraw();
	}

	/// Draw a character at a certain position. This writes pixels to the back buffer. In order to
	/// see changes on screen you must [Terminal::flush].
	fn draw_char(&mut self, character: Char, pos: CharPos) {
		let mut pixel_pos = pos.to_pixel();
		if character.character.is_ascii() {
			let ascii = character.character as usize;
			let char_bitmap = &FONT[ascii];
			let foreground: Pixel = Pixel::new(255, 255, 255);
			let background: Pixel = Pixel::new(0, 0, 0);
			let foreground_row = [foreground; 8];
			let background_row = [background; 8];
			for y in 0..16 {
				let index = self.screen.pos_to_index(&pixel_pos);
				let row = &mut self.screen.back[index..index + 8];
				row.copy_from_slice(&foreground_row);
				unsafe {
					let mask = &MASK_TABLE[char_bitmap[y] as usize];
					let row_as_u64 = &mut *(row as *mut [Pixel] as *mut [u64; 4]);

					for i in 0..4 {
						row_as_u64[i] = row_as_u64[i] & mask[i];
					}
				}
				pixel_pos.y += 1;
			}
		}
	}

	/// Redraw entire screen. Loop over every character and draw it.
	pub fn redraw(&mut self) {
		let chars = self.chars;
		for (y, line) in chars.iter().enumerate() {
			for (x, character) in line.iter().enumerate() {
				self.draw_char(*character, Vector::new(x, y))
			}
		}
	}

	/// Flush the [Screen] of this terminal. Uses [Screen::flush].
	fn flush(&mut self) {
		self.screen.flush()
	}
}
