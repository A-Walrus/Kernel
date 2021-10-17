// this is mostly stolen from ansi_term crate

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct Style {
	/// The style's foreground colour, if it has one.
	pub foreground: Option<Colour>,

	/// The style's background colour, if it has one.
	pub background: Option<Colour>,

	/// Whether this style is bold.
	pub is_bold: bool,

	/// Whether this style is dimmed.
	pub is_dimmed: bool,

	/// Whether this style is italic.
	pub is_italic: bool,

	/// Whether this style is underlined.
	pub is_underline: bool,

	/// Whether this style is blinking.
	pub is_blink: bool,

	/// Whether this style has reverse colours.
	pub is_reverse: bool,

	/// Whether this style is hidden.
	pub is_hidden: bool,

	/// Whether this style is struckthrough.
	pub is_strikethrough: bool,
}

impl Style {
	pub fn new() -> Style {
		Style::default()
	}

	pub fn bold(&self) -> Style {
		Style { is_bold: true, ..*self }
	}

	pub fn dimmed(&self) -> Style {
		Style {
			is_dimmed: true,
			..*self
		}
	}

	pub fn italic(&self) -> Style {
		Style {
			is_italic: true,
			..*self
		}
	}

	pub fn underline(&self) -> Style {
		Style {
			is_underline: true,
			..*self
		}
	}

	pub fn blink(&self) -> Style {
		Style {
			is_blink: true,
			..*self
		}
	}

	pub fn reverse(&self) -> Style {
		Style {
			is_reverse: true,
			..*self
		}
	}

	pub fn hidden(&self) -> Style {
		Style {
			is_hidden: true,
			..*self
		}
	}

	pub fn strikethrough(&self) -> Style {
		Style {
			is_strikethrough: true,
			..*self
		}
	}

	pub fn fg(&self, foreground: Colour) -> Style {
		Style {
			foreground: Some(foreground),
			..*self
		}
	}

	pub fn on(&self, background: Colour) -> Style {
		Style {
			background: Some(background),
			..*self
		}
	}

	pub fn is_plain(self) -> bool {
		self == Style::default()
	}
}

impl Default for Style {
	fn default() -> Style {
		Style {
			foreground: None,
			background: None,
			is_bold: false,
			is_dimmed: false,
			is_italic: false,
			is_underline: false,
			is_blink: false,
			is_reverse: false,
			is_hidden: false,
			is_strikethrough: false,
		}
	}
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Colour {
	/// Colour #0 (foreground code `30`, background code `40`).
	///
	/// This is not necessarily the background colour, and using it as one may
	/// render the text hard to read on terminals with dark backgrounds.
	Black,

	/// Colour #1 (foreground code `31`, background code `41`).
	Red,

	/// Colour #2 (foreground code `32`, background code `42`).
	Green,

	/// Colour #3 (foreground code `33`, background code `43`).
	Yellow,

	/// Colour #4 (foreground code `34`, background code `44`).
	Blue,

	/// Colour #5 (foreground code `35`, background code `45`).
	Purple,

	/// Colour #6 (foreground code `36`, background code `46`).
	Cyan,

	/// Colour #7 (foreground code `37`, background code `47`).
	///
	/// As above, this is not necessarily the foreground colour, and may be
	/// hard to read on terminals with light backgrounds.
	White,

	// Color from pallette
	Fixed(u8),

	/// A 24-bit RGB color, as specified by ISO-8613-3.
	RGB(u8, u8, u8),
}

impl Colour {
	pub fn normal(self) -> Style {
		Style {
			foreground: Some(self),
			..Style::default()
		}
	}
}

impl From<Colour> for Style {
	fn from(colour: Colour) -> Style {
		colour.normal()
	}
}
