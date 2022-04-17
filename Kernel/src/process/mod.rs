use alloc::vec::Vec;

/// Module for working with elf executables
pub mod elf;

type Pid = usize;

type OpenFiles = ();

enum State {
	New,
	Ready,
	Suspended,
}

/// Process control block
pub struct PCB {
	pid: Pid,
	state: State,
	page_table: (),
	registers: (),
	open_files: OpenFiles,
}
