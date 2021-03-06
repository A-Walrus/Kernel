#![no_std]
#![feature(asm)]
#![feature(const_for)] // for loops in const functions
#![feature(const_mut_refs)] // mutable references inside const functions
#![feature(alloc_error_handler)] // error handler for alloc failiures

pub mod syscalls;

pub mod io;

extern crate alloc;

macro_rules! syscall {
    ($($name:ident($a:ident, $($b:ident, $($c:ident, $($d:ident, $($e:ident, $($f:ident, )?)?)?)?)?);)+) => {
        $(
            pub unsafe fn $name(mut $a: i64, $($b: usize, $($c: usize, $($d: usize, $($e: usize, $($f: usize)?)?)?)?)?) -> i64 {
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

#[panic_handler]
pub fn panic(info: &core::panic::PanicInfo) -> ! {
	println!("{}", info);
	syscalls::exit(-1);
}

use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

fn init_heap() {
	let heap_start = 0x0000400000000000;
	let heap_size = 0x800000; // 8MiB
	unsafe {
		ALLOCATOR.lock().init(heap_start, heap_size);
	}
}

pub fn init() {
	init_heap();
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
	panic!("allocation error: {:?}", layout)
}

extern "C" {
	fn main() -> isize;
}

static mut ARGS: &[&str] = &[];

pub fn get_args() -> &'static [&'static str] {
	unsafe { ARGS }
}

#[no_mangle]
pub extern "C" fn _start(argc: usize, argv: *const &'static str) {
	init();
	// println!("argc: {}", argc);
	// println!("argv: {:?}", argv);
	let args: &[&str] = unsafe { core::slice::from_raw_parts(argv, argc) };
	unsafe { ARGS = args };
	// println!("args: {:?}", args);

	let result = unsafe { main() };
	syscalls::exit(result);
}
