#[repr(packed)]
#[derive(Copy, Clone)]
pub struct Pixel {
	pub blue: u8,
	pub green: u8,
	pub red: u8,
	pub padding: u8,
}
