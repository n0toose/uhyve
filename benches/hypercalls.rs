extern crate criterion;
extern crate uhyvelib;

use std::ffi::CString;

use criterion::{criterion_group, Criterion, Throughput};
use uhyve_interface::{
	parameters::{OpenParams, UnlinkParams},
	GuestPhysAddr,
};
use uhyvelib::{mem::MmapMemory, MIN_PHYSMEM_SIZE};

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

fn fill_memory_with_100_tempfile_names(
	mem: &MmapMemory,
	array_len: usize,
	name_len: usize,
) -> usize {
	let names: Vec<CString> = (0..array_len)
		.map(|i| {
			let string: String = format!("{}{}{}", "/tmp/", format!("{:0>3}", i), ".txt");
			CString::new(string.as_bytes()).unwrap()
		})
		.collect();
	assert_eq!(names[0].as_bytes_with_nul().len(), name_len);
	assert_eq!(names.len(), array_len);

	let mut mem_slice_mut = init_guest_mem(mem);

	// Making things nicer for the debugger.
	let offset = MIN_PHYSMEM_SIZE;
	for i in 0..array_len {
		let name = names[i].as_bytes_with_nul();
		for j in 0..name_len {
			mem_slice_mut[offset + i * name_len + j] = name[j];
		}
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

	let mut retcode = open.ret;
	let mut unlink: &mut UnlinkParams = &mut UnlinkParams {
		name: GuestPhysAddr::new(0),
		ret: 0,
	};

	let mut group: criterion::BenchmarkGroup<'_, criterion::measurement::WallTime> =
		c.benchmark_group("hypercall_open_unlink_test");
	group.sample_size(200);

	group.throughput(Throughput::Elements(name_len));
	group.bench_function("uhyve open() hypercall", |b| {
		b.iter(|| {
			for i in 0..ARRAY_LEN {
				let name = GuestPhysAddr::new((MIN_PHYSMEM_SIZE + NAME_LEN * i) as u64);
				open.name = name;
				uhyvelib::hypercall::open(&mem, &mut open);
				assert_ne!(retcode, -1);
				unlink.name = name;
				uhyvelib::hypercall::unlink(&mem, &mut unlink);
				retcode = unlink.ret;
				assert_ne!(retcode, -1);
			}
		});
	});
	group.finish();
}

criterion_group!(run_hypercalls_group, run_open_unlink_test);
