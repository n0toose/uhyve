extern crate criterion;

use criterion::criterion_main;

mod vm;
use crate::vm::load_kernel_benchmark_group;

mod complete_binary;
use crate::complete_binary::run_complete_binaries_group;

mod kernels;
use crate::kernels::run_kernel_group;

// Add the benchmark groups that should be run
criterion_main!(run_kernel_group);
