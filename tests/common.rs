use std::{
	env,
	fs::{read, remove_file},
	path::{Path, PathBuf},
	process::Command,
};

use byte_unit::{Byte, Unit};
use uhyvelib::{params::Params, vm::UhyveVm};

/// Uses Cargo to build a kernel in the `tests/test-kernels` directory.
/// Returns a path to the build binary.
pub fn build_hermit_bin(kernel: impl AsRef<Path>) -> PathBuf {
	let kernel = kernel.as_ref();
	println!("Building Kernel {}", kernel.display());
	let kernel_src_path = Path::new("tests/test-kernels");
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
	assert!(cmd.success(), "Test binaries could not be build");
	[
		kernel_src_path,
		Path::new("target/x86_64-unknown-hermit/debug"),
		Path::new(kernel),
	]
	.iter()
	.collect()
}

/// Wrapper around [`UhyveVm::new`] with default parameters for a small and
/// simple UhyveVM.
///
/// `kernel_path` - Location of the kernel
#[allow(dead_code)]
pub fn run_simple_vm(kernel_path: PathBuf) -> i32 {
	let params = Params {
		verbose: true,
		cpu_count: 2.try_into().unwrap(),
		memory_size: Byte::from_u64_with_unit(32, Unit::MiB)
			.unwrap()
			.try_into()
			.unwrap(),
		..Default::default()
	};

	UhyveVm::new(kernel_path, params).unwrap().run(None)
}

/// Small wrapper around [`UhyveVm::new`] that also accepts a filemap.
///
/// `kernel_path` - Location of the kernel
/// `file_map` - Vec<String> containing Strings of the format `host.txt:guest.txt`.
#[allow(dead_code)]
pub fn run_vm_with_file_map(kernel_path: PathBuf, file_map: Vec<String>) -> i32 {
	let params = Params {
		verbose: true,
		cpu_count: 2.try_into().unwrap(),
		memory_size: Byte::from_u64_with_unit(32, Unit::MiB)
			.unwrap()
			.try_into()
			.unwrap(),
		file_map: Some(file_map),
		..Default::default()
	};

	UhyveVm::new(kernel_path, params).unwrap().run(None)
}

/// Creates a file on the host OS, while attempting to remove the the file if
/// it already exists.
#[allow(dead_code)]
pub fn remove_file_if_exists(path: &PathBuf) {
	if path.exists() {
		println!("Removing existing file {}", path.display());
		remove_file(path).unwrap_or_else(|_| panic!("Can't remove {}", path.display()));
	}
}

/// Verifies that the file was successfully created on the host OS and contains
/// the right content.
#[allow(dead_code)]
pub fn verify_file_contents(testfile: &Path, exists: bool) {
	if exists {
		assert!(testfile.exists());
		let file_content = read(testfile.to_str().unwrap()).unwrap();
		assert_eq!(file_content, "Hello, world!".as_bytes());
	} else {
		assert!(!testfile.exists());
	}
}
