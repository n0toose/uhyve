extern crate criterion;
extern crate uhyvelib;

use std::ffi::CString;

use criterion::{criterion_group, Criterion, Throughput};
use uhyve_interface::{
	parameters::{OpenParams, UnlinkParams},
	GuestPhysAddr,
};
use uhyvelib::{isolation::UhyveFileMap, mem::MmapMemory, MIN_PHYSMEM_SIZE};
use uhyvelib::isolation::create_temp_dir;

/// Initialize the page tables for the guest
fn init_guest_mem(mem: &MmapMemory) -> &mut [u8] {
	let mem_slice_mut = unsafe { mem.as_slice_mut() };
	uhyvelib::init_guest_mem(
		mem_slice_mut
			.try_into()
			.expect("Guest memory is not large enough for pagetables"),
	);
	mem_slice_mut
}

fn string_to_cstring(string: String) -> CString {
	CString::new(string.as_bytes()).unwrap()
}

fn write_data_to_memory(mem: &MmapMemory, offset: usize, data: &[u8], data_size: usize) {
	let mut mem_slice_mut: &mut [u8] = init_guest_mem(mem);
	for i in 0..data_size {
		mem_slice_mut[offset + i] = data[i]
	}
}


/// Given a provided MmapMemory object, this function will write 100 names of the format
/// "/tmp/012.txt". It will return the length of the array, which should be 100.
/// 
/// TODO: Allow more than 100 names and different name lengths.
fn fill_memory_with_100_tempfile_names(
	mem: &MmapMemory,
	array_len: usize,
	name_len: usize,
) -> usize {
	let names: Vec<CString> = (0..array_len)
		.map(|i| {
			let string = format!("{}{}{}", "/tmp/", format!("{:0>3}", i), ".txt"); 
			string_to_cstring(string)
		})
		.collect();
	assert_eq!(names[0].as_bytes_with_nul().len(), name_len);
	assert_eq!(names.len(), array_len);

	for i in 0..array_len {
		let name = names[i].as_bytes_with_nul();
		let offset = MIN_PHYSMEM_SIZE + i * name_len;
		write_data_to_memory(&mem, offset, name, name_len);
	}

	names.len()
}

pub fn run_open_unlink_test(c: &mut Criterion) {
	const ARRAY_LEN: usize = 100;
	const NAME_LEN: usize = 13;

	let mem: MmapMemory =
		MmapMemory::new(0, MIN_PHYSMEM_SIZE * 2, GuestPhysAddr::new(0), false, true);

	// Example: "/tmp/012.txt"
	let name_len: u64 = fill_memory_with_100_tempfile_names(&mem, ARRAY_LEN, NAME_LEN) as u64;
	let mut open = &mut OpenParams {
		name: GuestPhysAddr::new(0),
		flags: 0o0001 | 0o0100 | 0o0200, // O_WRONLY|O_CREAT|O_EXCL
		mode: 0o0666,
		ret: 0,
	};

	let empty_array: [String; 0] = [];
	let tempdir = create_temp_dir();

	let mut retcode = open.ret;
	let mut unlink: &mut UnlinkParams = &mut UnlinkParams {
		name: GuestPhysAddr::new(0),
		ret: 0,
	};

	let mut group: criterion::BenchmarkGroup<'_, criterion::measurement::WallTime> =
		c.benchmark_group("uhyve hypercall: open + unlink");
	group.sample_size(200);

	group.throughput(Throughput::Elements(name_len * 100));
	group.bench_function("uhyve hypercall: open + unlink", |b| {
		b.iter(|| {
			for i in 0..ARRAY_LEN {
				let mut map: UhyveFileMap = UhyveFileMap::new(&empty_array);
				let name = GuestPhysAddr::new((MIN_PHYSMEM_SIZE + NAME_LEN * i) as u64);
				open.name = name;
				uhyvelib::hypercall::open(&mem, &mut open, &mut map, &tempdir);
				assert_ne!(retcode, -1);
				unlink.name = name;
				uhyvelib::hypercall::unlink(&mem, &mut unlink, &mut map,);
				retcode = unlink.ret;
				assert_ne!(retcode, -1);
			}
		});
	});
	group.finish();
}

pub fn run_open_test(c: &mut Criterion) {
	let mem: MmapMemory =
		MmapMemory::new(0, MIN_PHYSMEM_SIZE * 2, GuestPhysAddr::new(0), false, true);

	let tempdir = create_temp_dir();
	let path_str = "/dev/zero\0";
	write_data_to_memory(&mem, MIN_PHYSMEM_SIZE, path_str.as_bytes(), path_str.len());
	let mut open = &mut OpenParams {
		name: GuestPhysAddr::new(MIN_PHYSMEM_SIZE as u64),
		flags: 0o0001 | 0o0100 | 0o0200, // O_WRONLY|O_CREAT|O_EXCL
		mode: 0o0666,
		ret: 0,
	};

	let mut retcode = open.ret;

	let mut group: criterion::BenchmarkGroup<'_, criterion::measurement::WallTime> =
		c.benchmark_group("uhyve hypercall: open /dev/null");
	group.sample_size(2000);

	group.bench_function("empty file map", |b| {
		b.iter(|| {
			let mut map: UhyveFileMap = UhyveFileMap::new(&[]);
			uhyvelib::hypercall::open(&mem, &mut open, &mut map, &tempdir);
			retcode = open.ret;
			assert_eq!(retcode, -1);
		});
	});
	group.finish();

	group = c.benchmark_group("uhyve hypercall: open /dev/null");
	group.sample_size(2000);
	group.bench_function("empty file map (two opens)", |b| {
		b.iter(|| {
			let mut map: UhyveFileMap = UhyveFileMap::new(&[]);
			uhyvelib::hypercall::open(&mem, &mut open, &mut map, &tempdir);
			retcode = open.ret;
			assert_eq!(retcode, -1);
			uhyvelib::hypercall::open(&mem, &mut open, &mut map, &tempdir);
			retcode = open.ret;
			assert_eq!(retcode, -1);
		});
	});
	group.finish();

	group = c.benchmark_group("uhyve hypercall: open /dev/null");
	group.sample_size(2000);
	group.bench_function("file map containing file", |b| {
		b.iter(|| {
			// returns fd 0
			let mut map: UhyveFileMap = UhyveFileMap::new(&["/dev/zero:/dev/zero".to_string()]);
			uhyvelib::hypercall::open(&mem, &mut open, &mut map, &tempdir);
			retcode = open.ret;
			assert_eq!(retcode, -1);
		});
	});
	group.finish();

	group = c.benchmark_group("uhyve hypercall: open /dev/null");
	group.sample_size(200);
	group.bench_function("file map containing file (two opens)", |b| {
		b.iter(|| {
			// returns fd 0
			let mut map: UhyveFileMap = UhyveFileMap::new(&["/dev/zero:/dev/zero".to_string()]);
			uhyvelib::hypercall::open(&mem, &mut open, &mut map, &tempdir);
			retcode = open.ret;
			assert_eq!(retcode, -1);
			uhyvelib::hypercall::open(&mem, &mut open, &mut map, &tempdir);
			retcode = open.ret;
			assert_eq!(retcode, -1);
		});
	});
	group.finish();
}

criterion_group!(run_hypercalls_group, run_open_test);
