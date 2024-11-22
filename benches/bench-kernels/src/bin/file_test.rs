use std::{fs::File, fs::remove_file, io::prelude::*};

#[cfg(target_os = "hermit")]
use hermit as _;

fn main() {
	let mut file = File::create("/root/foo.txt").unwrap();
	file.write_all(b"Hello, world!").unwrap();
    remove_file("/root/foo.txt");
}
