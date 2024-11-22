extern crate criterion;
extern crate uhyvelib;

use std::{
	env, f64::MIN, fs, io::Write, path::{Path, PathBuf}, process::{Command, Stdio}
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
    ::core::slice::from_raw_parts(
        (p as *const T) as *const u8,
        ::core::mem::size_of::<T>(),
    )
}

pub fn run_open_test(c: &mut Criterion) {
	use uhyve_interface::parameters::OpenParams;
	
	use uhyvelib::mem::MmapMemory;
	use uhyvelib::MIN_PHYSMEM_SIZE;
	use uhyve_interface::GuestPhysAddr;
	use uhyvelib::initialize_pagetables;
	use std::ffi::{OsString, CString};
	use std::os::unix::ffi::OsStrExt;
	use log::error;

	let names: Vec<CString> = (0..100)
    	.map(|i| CString::new(OsString::from("/tmp/".to_owned() + i.to_string().as_str() + ".txt").as_bytes()).unwrap())
    	.collect();

	let mut mem: MmapMemory = MmapMemory::new(
		0,
		MIN_PHYSMEM_SIZE * 2,
		GuestPhysAddr::new(0),
		false,
		true
	);

	// First MIN_PHYSMEM_SIZE is allocated, mem is presumed to be zero.
	initialize_pagetables(unsafe { mem.as_slice_mut() }.try_into().unwrap());

	let name = names[0].as_bytes_with_nul().to_vec();
	let name_converted_back = CString::from_vec_with_nul(name.clone()).unwrap();
	error!("{:#?}", name_converted_back);

	unsafe {
		mem.as_slice_mut()[MIN_PHYSMEM_SIZE + 0x00] = name[0];
		mem.as_slice_mut()[MIN_PHYSMEM_SIZE + 0x01] = name[1];
		mem.as_slice_mut()[MIN_PHYSMEM_SIZE + 0x02] = name[2];
		mem.as_slice_mut()[MIN_PHYSMEM_SIZE + 0x03] = name[3];
		mem.as_slice_mut()[MIN_PHYSMEM_SIZE + 0x04] = name[4];
		mem.as_slice_mut()[MIN_PHYSMEM_SIZE + 0x05] = name[5];
		mem.as_slice_mut()[MIN_PHYSMEM_SIZE + 0x06] = name[6];
		mem.as_slice_mut()[MIN_PHYSMEM_SIZE + 0x07] = name[7];
		mem.as_slice_mut()[MIN_PHYSMEM_SIZE + 0x08] = name[8];
		mem.as_slice_mut()[MIN_PHYSMEM_SIZE + 0x09] = name[9];
		mem.as_slice_mut()[MIN_PHYSMEM_SIZE + 0x0a] = name[10];
	}

	let mut slice = unsafe{mem.as_slice_mut()}.to_vec();
	for i in 0..slice.len() {
		if (slice[i] == name[0] && slice[i+1] == name[1]) {
			panic!("it works! {}, {}", i, i+1);
		}
	}
	println!("test");
	println!("test");
	println!("test");
	println!("test");
	/*
	let firstopenparam: &mut OpenParams = &mut OpenParams {
		name: GuestPhysAddr::new(unsafe { mem.host_address.add((MIN_PHYSMEM_SIZE as u8) as usize)} as u64),
		flags: 0o100 | 0o2,
		mode: 0x777,
		ret: 0
	};
	uhyvelib::hypercall::open(&mem, firstopenparam);
	let retcode = firstopenparam.ret;
*/
	/*
	let first_openparam = unsafe {any_as_u8_slice(openparams_vec.into_iter().next().unwrap())}.to_vec();
	let size_first_openparam: usize = first_openparam.len() * size_of::<u8>();
	let second_openparam = unsafe {any_as_u8_slice(openparams_vec.into_iter().next().unwrap())}.to_vec();
	let size_second_openparam: usize = second_openparam.len() * size_of::<u8>();
	env_logger::builder().is_test(true).try_init().unwrap();
	error!("{}", size_first_openparam);
	error!("{}", size_second_openparam);
	error!("{}", std::mem::size_of::<OpenParams>());
	let mut slice = unsafe { mem.slice_at_mut(GuestPhysAddr::new(MIN_PHYSMEM_SIZE as u64), size_first_openparam) }.unwrap();
	slice.write(&first_openparam).unwrap();
	uhyvelib::hypercall::open(&mem, openparam);
	*/
	/*
	let mut group = c.benchmark_group("compile_hello_world");
	group.sample_size(100);
	group.throughput(Throughput::Elements(4096 as u64));

	for i in 0..100 {
		group.bench_with_input(BenchmarkId::new("uhyve open() test", i), &i,
			|b| {
				let openparam = &mut openparams_vec.into_iter().next().unwrap();
				uhyvelib::hypercall::open(&mem, openparam);
				let fd = openparam.ret;
				let close: &mut CloseParams = &mut CloseParams { fd: fd, ret: -1 };
				uhyvelib::hypercall::close(close);
			});
	}
	*/
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
	assert!(hello_world_pathbuf.exists(), "compiled hello_world kernel missing");
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
	assert!(file_test_pathbuf.exists(), "compiled file_test kernel missing");
	let file_test_path = file_test_pathbuf.into_os_string().into_string().unwrap();

	let mut group = c.benchmark_group("run_file_test");
	group.sample_size(100);

	group.bench_function(
		"uhyve file_test",
		|b| {
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
		},
	);
}

// criterion_group!(run_kernel_group, run_compile_hello_world, run_file_test);
criterion_group!(run_kernel_group, run_open_test);
