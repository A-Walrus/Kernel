#![no_std]
#![no_main]
#![feature(asm)]
#![feature(naked_functions)]

extern crate alloc;
use bootloader::{entry_point, BootInfo};
use kernel::{
	cpu::{
		gdt::{self, GDT},
		interrupts,
	},
	fs::ext2,
	io::buffer,
	mem::{buddy, heap, paging},
	serial_println,
};
use x86_64::{instructions::segmentation::DS, registers::rflags::RFlags};

entry_point!(kernel_main);

#[naked]
fn test_function() {
	unsafe {
		asm!(
			"2:",
			"nop",
			"nop",
			"mov rax, 0xCA11",
			"syscall",
			"jmp 2b",
			options(noreturn)
		);
	}
}

#[naked]
fn handle_syscall() {
	unsafe {
		asm!("nop", "nop", "nop", "sysretq", options(noreturn));
	}
}

/// Entry point for the kernel. Returns [!] because it is never supposed to exit.
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
	if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
		gdt::setup();
		paging::setup();
		buddy::setup(&boot_info.memory_regions);
		heap::setup(buffer::calc_real_length(framebuffer));
		interrupts::setup();
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
				end: Page::containing_address(VirtAddr::new(ADDR + 4096)),
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

		// SETUP FOR RING 3
		{
			use x86_64::registers::model_specific::*;
			let cs_sysret = GDT.1.user_code_selector;
			let ss_sysret = GDT.1.user_data_selector;
			let cs_syscall = GDT.1.kernel_code_selector;
			let ss_syscall = GDT.1.kernel_data_selector;
			Star::write(cs_sysret, ss_sysret, cs_syscall, ss_syscall).expect("Failed to write star");

			unsafe {
				Efer::write(Efer::read() | EferFlags::SYSTEM_CALL_EXTENSIONS);
			}

			LStar::write(VirtAddr::from_ptr(handle_syscall as *const u8));
			SFMask::write(RFlags::INTERRUPT_FLAG);
		}

		unsafe {
			go_to_ring3(ADDR, ADDR + 4000);
		}

		// unsafe {
		// 	asm!(
		// "jmp {function}",
		// function = in(reg) ADDR);
		// }

		// ext2::cleanup().expect("Failed to cleanup EXT2");
		serial_println!("Finished cleanup");
	}
	serial_println!("The end");
	loop {}
}

unsafe fn go_to_ring3(code: u64, stack_end: u64) {
	let cs_idx: u16 = GDT.1.user_code_selector.0;
	let ds_idx: u16 = GDT.1.user_data_selector.0;

	use x86_64::instructions::segmentation::Segment;
	x86_64::instructions::tlb::flush_all();
	DS::set_reg(GDT.1.user_data_selector);
	asm!(
	"push rax",
	"push rsi",
	"push 0x200",
	"push rdx",
	"push rdi",
	"iretq",
	in("rdi") code,
	in("rsi") stack_end,
	in("dx") cs_idx,
	in("ax") ds_idx,
	);
}
