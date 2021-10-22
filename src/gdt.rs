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
	static ref GDT: (GlobalDescriptorTable, Selectors) = {
		let mut gdt = GlobalDescriptorTable::new();
		let kernel_code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
		let kernel_data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
		let user_code_selector = gdt.add_entry(Descriptor::user_data_segment());
		let user_data_selector = gdt.add_entry(Descriptor::user_data_segment());
		let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
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
struct Selectors {
	kernel_code_selector: SegmentSelector,
	kernel_data_selector: SegmentSelector,
	user_code_selector: SegmentSelector,
	user_data_selector: SegmentSelector,
	tss_selector: SegmentSelector,
}

pub fn setup() {
	unsafe {
		GDT.0.load();
		CS::set_reg(GDT.1.kernel_code_selector);
		load_tss(GDT.1.tss_selector);
	}
}
