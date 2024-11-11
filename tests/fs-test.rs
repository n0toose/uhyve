mod common;

use std::{fs::remove_file, path::PathBuf};

use common::{build_hermit_bin, remove_file_if_exists, run_simple_vm, verify_file_contents};

#[test]
fn new_file_test() {
	let testfile = PathBuf::from("foo.txt");
	remove_file_if_exists(&testfile);
	let bin_path = build_hermit_bin("create_file");

	assert_eq!(0, run_simple_vm(bin_path));
	verify_file_contents(&testfile, true);
	remove_file(&testfile).unwrap_or_else(|_| panic!("Can't remove {}", testfile.display()));
}
