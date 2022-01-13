use super::pci;

/// Setup AHCI
pub fn setup() {
	let functions = pci::recursive_scan();
	let res = functions
		.iter()
		.find(|func| pci::get_class_code(**func) == 0x01 && pci::get_subclass_code(**func) == 0x06);
	match res {
		Some(function) => {
			serial_println!("Found AHCI device: {:?}", function);
		}
		None => {
			serial_println!("No AHCI device, cannot access storage!");
		}
	}
}
