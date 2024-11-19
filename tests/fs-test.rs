mod common;

use byte_unit::{Byte, Unit};
use common::{build_hermit_bin, verify_file_contents};
use uhyvelib::{params::Params, vm::UhyveVm};

#[test]
fn new_file_test() {
	let params = Params {
		verbose: true,
		cpu_count: 2.try_into().unwrap(),
		memory_size: Byte::from_u64_with_unit(32, Unit::MiB)
			.unwrap()
			.try_into()
			.unwrap(),
		..Default::default()
	};

	let bin_path = build_hermit_bin("create_file");
	let vm = UhyveVm::new(bin_path, params).unwrap();
	let mut testfile = vm.get_tempdir().path().to_path_buf();
	testfile.push("foo.txt");
	assert_eq!(0, vm.run(None));
	verify_file_contents(&testfile);
}
