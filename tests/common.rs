use std::{
	env,
	fs::remove_file,
	path::{Path, PathBuf},
	process::Command,
};

use byte_unit::{Byte, Unit};
use uhyvelib::{
	params::{Output, Params},
	vm::{UhyveVm, VmResult},
};

/// Uses Cargo to build a kernel in the `tests/test-kernels` directory.
/// Returns a path to the build binary.
pub fn build_hermit_bin(kernel: impl AsRef<Path>) -> PathBuf {
	let kernel = kernel.as_ref();
	let kernel_src_path = Path::new("tests/test-kernels");
	println!("Building test kernel: {}", kernel.display());

	let cmd = Command::new("cargo")
		.arg("build")
		.arg("-Zbuild-std=std,panic_abort")
		.arg("--target=x86_64-unknown-hermit")
		.arg("--bin")
		.arg(kernel)
		// Remove environment variables related to the current cargo instance (toolchain version, coverage flags)
		.env_clear()
		// Retain PATH since it is used to find cargo and cc
		.env("PATH", env::var_os("PATH").unwrap())
		.current_dir(kernel_src_path)
		.status()
		.expect("failed to execute `cargo build`");

	assert!(cmd.success(), "Test binaries could not be built.");
	[
		kernel_src_path,
		Path::new("target/x86_64-unknown-hermit/debug"),
		Path::new(kernel),
	]
	.iter()
	.collect()
}

/// Small wrapper around [`Uhyve::run`] with default parameters for a small and
/// simple Uhyve vm
#[allow(dead_code)]
pub fn run_simple_vm(kernel_path: PathBuf) -> VmResult {
	env_logger::try_init().ok();
	println!("Launching kernel {}", kernel_path.display());
	let params = Params {
		cpu_count: 2.try_into().unwrap(),
		memory_size: Byte::from_u64_with_unit(32, Unit::MiB)
			.unwrap()
			.try_into()
			.unwrap(),
		output: Output::Buffer,
		stats: true,
		..Default::default()
	};

	UhyveVm::new(kernel_path, params).unwrap().run(None)
}

#[allow(dead_code)]
pub fn remove_file_if_exists(path: &PathBuf) {
	if path.exists() {
		println!("Removing existing directory {}", path.display());
		remove_file(path).unwrap_or_else(|_| panic!("Can't remove {}", path.display()));
	}
}
