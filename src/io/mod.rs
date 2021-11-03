/// Working with framebuffers and character buffers.
pub mod buffer;
/// Array describing a bitmap font.
pub mod font;
/// Dealing with a (PS/2) keyboard.
pub mod keyboard;
/// Array of masks for faster text rendering.
mod mask_table;

/// Sending and reading data from the serial port for debugging.
#[macro_use]
pub mod serial;
