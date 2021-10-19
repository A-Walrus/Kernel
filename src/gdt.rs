use lazy_static::lazy_static;
use x86_64::{
	self,
	structures::gdt::{Descriptor, GlobalDescriptorTable},
};

lazy_static! {
	static ref GDT: GlobalDescriptorTable = {
		let mut gdt = GlobalDescriptorTable::new();
		let kernel_code_segment = gdt.add_entry(Descriptor::kernel_code_segment());
		let kernel_data_segment = gdt.add_entry(Descriptor::kernel_data_segment());
		let user_code_segment = gdt.add_entry(Descriptor::user_data_segment());
		let user_data_segment = gdt.add_entry(Descriptor::user_data_segment());
		gdt
	};
}

pub fn setup() {
	unsafe {
		GDT.load();
	}
}
