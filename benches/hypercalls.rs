extern crate criterion;
extern crate uhyvelib;

use std::ffi::CString;

use criterion::{criterion_group, Criterion, Throughput};
use uhyve_interface::{
	parameters::{OpenParams, UnlinkParams},
	GuestPhysAddr,
};
use uhyvelib::{initialize_pagetables, mem::MmapMemory, MIN_PHYSMEM_SIZE};

pub fn run_open_unlink_test(c: &mut Criterion) {
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
					flags: 0o0001 | 0o0100 | 0o0200, // O_WRONLY|O_CREAT|O_EXCL
					mode: 0o0666,
					ret: 0,
				};
				uhyvelib::hypercall::open(&mem, &mut open);
				let mut retcode = open.ret;
				assert_ne!(retcode, -1);
				let mut unlink = &mut UnlinkParams { name: name, ret: 0 };
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

criterion_group!(run_hypercalls_group, run_open_unlink_test);
