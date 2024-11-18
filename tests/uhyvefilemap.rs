mod common;

use std::{fs::remove_file, path::PathBuf};

use common::{build_hermit_bin, remove_path_if_exists, run_vm_with_file_map, verify_file_contents};

#[test]
fn uhyvefilemap_test() {
	let testfile: PathBuf = PathBuf::from("foo.txt");
	remove_path_if_exists(&testfile);
	let bin_path = build_hermit_bin("create_file");

	// The file should not exist on the host OS.
	let mut code = run_vm_with_file_map(bin_path.clone(), vec!["foo.txt:wrong.txt".to_string()]);
	assert_eq!(0, code);
	verify_file_contents(&testfile, false);

	code = run_vm_with_file_map(bin_path, vec!["foo.txt:foo.txt".to_string()]);
	assert_eq!(0, code);
	verify_file_contents(&testfile, true);
	remove_file(&testfile).unwrap_or_else(|_| panic!("Can't remove {}", testfile.display()));
}
