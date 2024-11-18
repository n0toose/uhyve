mod common;

use std::{
	fs::{create_dir, remove_dir_all},
	path::PathBuf,
};

use common::{build_hermit_bin, remove_path_if_exists, run_vm_with_tempdir, verify_file_contents};

#[test]
fn new_file_test() {
	let tempdir = PathBuf::from("./tests/data/tmp");
	let testfile = tempdir.join("testprefix").join("foo.txt");
	remove_path_if_exists(&tempdir);
	create_dir(&tempdir).unwrap();

	let bin_path = build_hermit_bin("create_file");
	assert_eq!(0, run_vm_with_tempdir(bin_path, tempdir.to_owned()));
	verify_file_contents(&testfile, true);
	remove_dir_all(&tempdir).unwrap_or_else(|_| panic!("Can't remove {}", tempdir.display()));
}
