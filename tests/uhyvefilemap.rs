mod common;

use std::{fs::remove_file, path::PathBuf};

use byte_unit::{Byte, Unit};
use common::{build_hermit_bin, verify_file_contents};
use uhyvelib::{params::Params, vm::UhyveVm};

pub fn remove_path_if_exists(file: &PathBuf) {
	if file.exists() {
		println!("Removing existing file {}", file.display());
		// Also removes files.
	}
}

#[test]
fn uhyvefilemap_test() {
	let bin_path = build_hermit_bin("create_file");

	let params_wrong_file_map = Params {
		verbose: true,
		cpu_count: 2.try_into().unwrap(),
		memory_size: Byte::from_u64_with_unit(32, Unit::MiB)
			.unwrap()
			.try_into()
			.unwrap(),
		mount: Some(vec!["foo.txt:wrong.txt".to_string()]),
		..Default::default()
	};

	let params_right_file_map = Params {
		verbose: true,
		cpu_count: 2.try_into().unwrap(),
		memory_size: Byte::from_u64_with_unit(32, Unit::MiB)
			.unwrap()
			.try_into()
			.unwrap(),
		mount: Some(vec!["foo.txt:foo.txt".to_string()]),
		..Default::default()
	};

	// The file should exist in a temporary directory
	let mut vm = UhyveVm::new(bin_path.clone(), params_wrong_file_map).unwrap();
	let mut testfile = vm.get_tempdir().path().to_path_buf();
	testfile.push("foo.txt");
	assert_eq!(0, vm.run(None));
	verify_file_contents(&testfile);

	vm = UhyveVm::new(bin_path, params_right_file_map).unwrap();
	testfile = PathBuf::from("foo.txt");
	assert_eq!(0, vm.run(None));
	verify_file_contents(&testfile);
	remove_file(&testfile).unwrap_or_else(|_| panic!("Can't remove {}", testfile.display()));
}
