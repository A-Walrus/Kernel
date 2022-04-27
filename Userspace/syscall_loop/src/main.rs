#![no_std]
#![no_main]
#![feature(asm)]

macro_rules! syscall {
    ($($name:ident($a:ident, $($b:ident, $($c:ident, $($d:ident, $($e:ident, $($f:ident, )?)?)?)?)?);)+) => {
        $(
            pub unsafe fn $name(mut $a: usize, $($b: usize, $($c: usize, $($d: usize, $($e: usize, $($f: usize)?)?)?)?)?) -> usize {
                asm!(
                    "syscall",
                    inout("rax") $a,
                    $(
                        in("rdi") $b,
                        $(
                            in("rsi") $c,
                            $(
                                in("rdx") $d,
                                $(
                                    in("r10") $e,
                                    $(
                                        in("r8") $f,
                                    )?
                                )?
                            )?
                        )?
                    )?
                    out("rcx") _,
                    out("r11") _,
                    options(nostack),
                );

                $a
            }
        )+
    };
}

syscall! {
	syscall0(a,);
	syscall1(a, b,);
	syscall2(a, b, c,);
	syscall3(a, b, c, d,);
	syscall4(a, b, c, d, e,);
	syscall5(a, b, c, d, e, f,);
}

fn print(s: &str) {
	unsafe {
		syscall2(1, s.as_ptr() as usize, s.len());
	}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
	loop {
		print("B1 ");
		print("B2 ");
		print("B3 ");
		print("B4 ");
	}
}

use core::panic::PanicInfo;
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
	loop {}
}
