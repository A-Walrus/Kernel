/// Print a string to the console
pub fn print(s: &str) -> i64 {
	let addr = s.as_ptr();
	let len = s.len();
	let result: i64;
	unsafe {
		asm!(
			"mov rax, 0x1", // sys print
			"syscall",
			in("rdi") addr,
			in("rsi") len,
			out("rax") result,
		);
	}
	result
}
