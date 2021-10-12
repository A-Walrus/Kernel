pub const SCREEN_SIZE: usize = 480256;

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

type Buffer<'a> = &'a mut [Pixel];

struct ScreenInfo {}

pub struct Screen<'a> {
	pub front: Buffer<'a>,
	pub back: Buffer<'a>,
	info: ScreenInfo,
}

impl<'a> Screen<'a> {
	pub fn new(front: Buffer<'a>, back: Buffer<'a>) -> Self {
		Screen {
			front,
			back,
			info: ScreenInfo {},
		}
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
