use lazy_static::lazy_static;
use x86_64::{
	self,
	instructions::{segmentation::*, tables::load_tss},
	structures::{
		gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
		tss::TaskStateSegment,
	},
};

lazy_static! {
	static ref TSS: TaskStateSegment = {
		let tss = TaskStateSegment::new();
		tss
	};
}

lazy_static! {
	/// GDT
	pub static ref GDT: (GlobalDescriptorTable, Selectors) = {
		let mut gdt = GlobalDescriptorTable::new();
		let kernel_code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
		let kernel_data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
		let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
		let user_data_selector = gdt.add_entry(Descriptor::user_data_segment());
		let user_code_selector = gdt.add_entry(Descriptor::user_code_segment());
		(
			gdt,
			Selectors {
				kernel_code_selector,
				kernel_data_selector,
				user_code_selector,
				user_data_selector,
				tss_selector,
			},
		)
	};
}

#[allow(dead_code)]
/// All the data created statically to describe the GDT, and its segments.
pub struct Selectors {
	/// Kernel code selector
	pub kernel_code_selector: SegmentSelector,
	/// Kernel data selector
	pub kernel_data_selector: SegmentSelector,
	/// User code selector
	pub user_code_selector: SegmentSelector,
	/// User data selector
	pub user_data_selector: SegmentSelector,
	/// TSS selector
	pub tss_selector: SegmentSelector,
}

/// Set up global descriptor table, and set code segment register, and task
/// state segment register.
pub fn setup() {
	unsafe {
		GDT.0.load();
		CS::set_reg(GDT.1.kernel_code_selector);
		DS::set_reg(GDT.1.kernel_data_selector);
		load_tss(GDT.1.tss_selector);
	}
}
