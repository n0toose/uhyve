extern crate criterion;

use std::{
	env, fs,
	path::{Path, PathBuf},
	process::{Command, Stdio},
};

use criterion::{criterion_group, Criterion};

/// Uses Cargo to build a kernel in the `tests/test-kernels` directory.
/// Returns a path to the build binary.
pub fn build_hermit_bin(kernel: impl AsRef<Path>) -> PathBuf {
	let kernel = kernel.as_ref();
	let kernel_src_path = Path::new("benches/bench-kernels");
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

	assert!(cmd.success(), "Bench binaries could not be built.");
	[
		kernel_src_path,
		Path::new("target/x86_64-unknown-hermit/debug"),
		Path::new(kernel),
	]
	.iter()
	.collect()
}

// based on https://stackoverflow.com/questions/35045996/check-if-a-command-is-in-path-executable-as-process#35046243
fn is_program_in_path(program: &str) -> bool {
	if let Ok(path) = env::var("PATH") {
		for p in path.split(':') {
			let p_str = format!("{p}/{program}");
			if fs::metadata(p_str).is_ok() {
				return true;
			}
		}
	}
	false
}

pub fn run_compile_hello_world(c: &mut Criterion) {
	let uhyve_path = [env!("CARGO_MANIFEST_DIR"), "target/release/uhyve"]
		.iter()
		.collect::<PathBuf>();
	assert!(
		uhyve_path.exists(),
		"uhyve release build is required to run this benchmark"
	);

	let hello_world_path = [env!("CARGO_MANIFEST_DIR"), "benches/bench-kernels/target/x86_64-unknown-hermit/debug/hello_world"]
	.iter()
	.collect::<PathBuf>();
	assert!(
		hello_world_path.exists(),
		"hello_world executable missing from bench_data"
	);
	/*
	// Unlike in complete_binary.rs, this dark magic is necessary because of ownership. (???)
	let hello_world_pathbuf = build_hermit_bin("hello_world");
	assert!(hello_world_pathbuf.exists(), "compiled hello_world kernel missing");
	let hello_world_path = hello_world_pathbuf.into_os_string().into_string().unwrap();
	*/
	let mut group = c.benchmark_group("compile_hello_world");
	group.sample_size(10);

	group.bench_function("uhyve target/debug/x86_64-unknown-hermit/debug/hello_world", |b| {
		b.iter(|| {
			let status = Command::new(&uhyve_path)
				.arg(&hello_world_path)
				.arg("-m")
				.arg("64MiB")
				.stdout(Stdio::null())
				.status()
				.expect("failed to execute process");
			assert!(status.success());
		})
	});

	let qemu_available = is_program_in_path("qemu-system-x86_64");

	if !qemu_available {
		println!("qemu-system-x86_64 not found in path, skipping QEMU benchmark");
		return;
	}

	let loader_path = [env!("CARGO_MANIFEST_DIR"), "hermit-loader-x86_64"]
		.iter()
		.collect::<PathBuf>();
	assert!(
		loader_path.exists(),
		"hermit-loader-x86_64 was not found in {}, please download it manually and try again",
		loader_path.into_os_string().into_string().unwrap()
	);

	group.bench_function("qemu target/debug/x86_64-unknown-hermit/debug/hello_world", |b| {
		b.iter(|| {
			let status = Command::new("qemu-system-x86_64")
				.arg("-smp")
				.arg("1")
				.arg("-m")
				.arg("64M")
				.arg("-kernel")
				.arg(&loader_path)
				.arg("-initrd")
				.arg(&hello_world_path)
				.arg("-display")
				.arg("none")
				.arg("-serial")
				.arg("stdio")
				.arg("-enable-kvm")
				.arg("-cpu")
				.arg("host")
				.stdout(Stdio::null())
				.status()
				.expect("failed to execute process");
			assert!(status.success());
		})
	});
}

criterion_group!(run_kernel_group, run_compile_hello_world);
