#![no_main]
#![no_std]

use standard::{get_args, println};
extern crate alloc;

#[no_mangle]
pub extern "C" fn main() -> isize {
	let args = get_args();
	let message = args.get(0).unwrap_or(&"MOO");
	let line = "-".repeat(message.len() + 4);
	println!(" {0}\n | {1} |\n {0}{2}", line, message, COW);

	return 0;
}

const COW: &str = r#"
   \   ^__^
    \  (oo)\_______
       (__)\       )\/\
           ||----w |
           ||     ||"#;
