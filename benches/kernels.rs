extern crate criterion;
extern crate uhyvelib;

use std::{
	array, env,
	f64::MIN,
	fs::{self, remove_file},
	io::Write,
	path::{Path, PathBuf},
	process::{Command, Stdio},
};

use criterion::{criterion_group, Criterion};
use uhyve_interface::parameters::{CloseParams, UnlinkParams};

/// Uses Cargo to build a kernel in the `tests/test-kernels` directory.
/// Returns a path to the build binary.
pub fn build_hermit_bin(kernel: impl AsRef<Path>) -> PathBuf {
	let kernel = kernel.as_ref();
	let kernel_src_path = Path::new("benches/bench-kernels");
	println!("Building test kernel: {}", kernel.display());

	let cmd = Command::new("cargo")
		.arg("build")
		.arg("--release")
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
		Path::new("target/x86_64-unknown-hermit/release"),
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

// https://stackoverflow.com/questions/28127165/how-to-convert-struct-to-u8/42186553
unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
	::core::slice::from_raw_parts((p as *const T) as *const u8, ::core::mem::size_of::<T>())
}

pub fn run_open_unlink_test(c: &mut Criterion) {
	use std::ffi::CString;

	use criterion::Throughput;
	use uhyve_interface::{parameters::OpenParams, GuestPhysAddr};
	use uhyvelib::{initialize_pagetables, mem::MmapMemory, MIN_PHYSMEM_SIZE};

	const ARRAY_LEN: usize = 100;
	const NAME_LEN: usize = 13;

	let mem: MmapMemory =
		MmapMemory::new(0, MIN_PHYSMEM_SIZE * 2, GuestPhysAddr::new(0), false, true);
	// First MIN_PHYSMEM_SIZE is allocated, mem is presumed to be zero.
	let mem_slice_mut = unsafe { mem.as_slice_mut() };
	initialize_pagetables(mem_slice_mut.try_into().unwrap());

	// Example: "/tmp/012.txt"
	let names: Vec<CString> = (0..ARRAY_LEN)
		.map(|i| {
			let string: String = format!("{}{}{}", "/tmp/", format!("{:0>3}", i), ".txt");
			CString::new(string.as_bytes()).unwrap()
		})
		.collect();
	assert_eq!(names[0].as_bytes_with_nul().len(), NAME_LEN);
	assert_eq!(names.len(), ARRAY_LEN);

	// Making things nicer for the debugger.
	// TODO: Use .as_chunks() to split &[u8] into &[u8; NAME_LEN] once it turns stable.
	let offset = MIN_PHYSMEM_SIZE;
	for i in 0..ARRAY_LEN {
		let name = names[i].as_bytes_with_nul();
		for j in 0..NAME_LEN {
			mem_slice_mut[offset + i * NAME_LEN + j] = name[j];
		}
	}

	let mut group: criterion::BenchmarkGroup<'_, criterion::measurement::WallTime> =
		c.benchmark_group("hypercall_open_unlink_test");
	group.sample_size(200);

	group.throughput(Throughput::Elements(names.len() as u64));
	group.bench_function("uhyve open() hypercall", |b| {
		b.iter(|| {
			for i in 0..ARRAY_LEN {
				let name = GuestPhysAddr::new((MIN_PHYSMEM_SIZE + NAME_LEN * i) as u64);
				let mut open = &mut OpenParams {
					name: name,
					flags: 0o0001|0o0100|0o0200, // O_WRONLY|O_CREAT|O_EXCL
					mode: 0o0666,
					ret: 0,
				};
				uhyvelib::hypercall::open(&mem, &mut open);
				let mut retcode = open.ret;
				assert_ne!(retcode, -1);
				let mut unlink = &mut UnlinkParams {
					name: name,
					ret: 0,
				};
				uhyvelib::hypercall::unlink(&mem, &mut unlink);
				retcode = unlink.ret;
				assert_ne!(retcode, -1);
			}
		});

		//for i in 0..ARRAY_LEN {
			//remove_file(format!("{}{}{}", "/tmp/", format!("{:0>3}", i), ".txt")).unwrap();
		//}
	});
	group.finish();
}

pub fn run_compile_hello_world(c: &mut Criterion) {
	let uhyve_path = [env!("CARGO_MANIFEST_DIR"), "target/release/uhyve"]
		.iter()
		.collect::<PathBuf>();
	assert!(
		uhyve_path.exists(),
		"uhyve release build is required to run this benchmark"
	);

	// Unlike in complete_binary.rs, this dark magic is necessary because of ownership. (???)
	let hello_world_pathbuf = build_hermit_bin("hello_world");
	assert!(
		hello_world_pathbuf.exists(),
		"compiled hello_world kernel missing"
	);
	let hello_world_path = hello_world_pathbuf.into_os_string().into_string().unwrap();

	let mut group = c.benchmark_group("compile_hello_world");
	group.sample_size(100);

	group.bench_function(
		"uhyve target/debug/x86_64-unknown-hermit/debug/hello_world",
		|b| {
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
		},
	);
}

pub fn run_file_test(c: &mut Criterion) {
	let uhyve_path = [env!("CARGO_MANIFEST_DIR"), "target/release/uhyve"]
		.iter()
		.collect::<PathBuf>();
	assert!(
		uhyve_path.exists(),
		"uhyve release build is required to run this benchmark"
	);

	// Unlike in complete_binary.rs, this dark magic is necessary because of ownership. (???)
	let file_test_pathbuf = build_hermit_bin("file_test");
	assert!(
		file_test_pathbuf.exists(),
		"compiled file_test kernel missing"
	);
	let file_test_path = file_test_pathbuf.into_os_string().into_string().unwrap();

	let mut group = c.benchmark_group("run_file_test");
	group.sample_size(100);

	group.bench_function("uhyve file_test", |b| {
		b.iter(|| {
			let status = Command::new(&uhyve_path)
				.arg(&file_test_path)
				.arg("-m")
				.arg("64MiB")
				.stdout(Stdio::null())
				.status()
				.expect("failed to execute process");
			assert!(status.success());
		})
	});
}

// criterion_group!(run_kernel_group, run_compile_hello_world, run_file_test);
criterion_group!(run_kernel_group, run_open_unlink_test);
