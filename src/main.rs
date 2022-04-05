#![no_std]
#![no_main]
#![feature(asm)]
#![feature(naked_functions)]

extern crate alloc;
use bootloader::{entry_point, BootInfo};
use kernel::{
	cpu::{gdt, interrupts, syscalls},
	fs::ext2,
	io::buffer,
	mem::{buddy, heap, paging},
	serial_println,
};

entry_point!(kernel_main);

/// Entry point for the kernel. Returns [!] because it is never supposed to exit.
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
		gdt::setup();
		paging::setup();
		buddy::setup(&boot_info.memory_regions);
		heap::setup(buffer::calc_real_length(framebuffer));
		interrupts::setup();
		syscalls::setup();
		let screen = buffer::Screen::new_from_framebuffer(framebuffer);
		let term = buffer::Terminal::new(screen);
		unsafe {
			buffer::TERM = Some(term);
		}
		// ext2::setup().expect("Failed to setup EXT2");
		serial_println!("Finished setup");

		use x86_64::{
			addr::VirtAddr,
			structures::paging::{
				page::{Page, PageRangeInclusive},
				PageTableFlags, Size4KiB,
			},
		};

		const ADDR: u64 = 0x400000;
		// Map userspace
		{
			let range = PageRangeInclusive::<Size4KiB> {
				start: Page::containing_address(VirtAddr::new(ADDR)),
				end: Page::containing_address(VirtAddr::new(ADDR + (2 * 4096))),
			};
			let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
			paging::map_in_current(range, flags);

			unsafe {
				let from = test_function as *const u8;
				let to = ADDR as *mut u8;
				to.copy_from(from, 4096);
			}

			// paging::print_table_recursive(paging::get_current_page_table(), 4);
		}

		unsafe {
			syscalls::go_to_ring3(VirtAddr::new(ADDR), VirtAddr::new(ADDR + 4096 + 4000));
		}

		// ext2::cleanup().expect("Failed to cleanup EXT2");
		serial_println!("Finished cleanup");
	}
	serial_println!("The end");
	loop {}
}

#[naked]
extern "C" fn test_function() {
	unsafe {
		asm!(
			"2:",
			"nop",
			"nop",
			"mov rax, 0x0",
			"mov rbx, 0x66",
			"mov rdi, 0xBB",
			"mov rsi, 0xCC",
			"mov rdx, 0xDD",
			"syscall",
			"jmp 2b",
			options(noreturn)
		);
	}
}
