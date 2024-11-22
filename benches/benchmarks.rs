extern crate criterion;

use criterion::criterion_main;

mod vm;

mod complete_binary;

mod kernels;
use crate::kernels::run_kernel_group;

// Add the benchmark groups that should be run
// criterion_main!(run_kernel_group, run_complete_binaries_group, load_kernel_benchmark_group);
criterion_main!(run_kernel_group);