use sysinfo::System;
use thiserror::Error;

#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use self::x86_64::*;

#[cfg(target_arch = "aarch64")]
pub mod aarch64;

#[cfg(target_arch = "aarch64")]
pub use self::aarch64::*;

#[derive(Error, Debug)]
#[error("Frequency detection failed")]
pub struct FrequencyDetectionFailed;

pub fn detect_freq_from_sysinfo() -> std::result::Result<u32, FrequencyDetectionFailed> {
	debug!("Trying to detect CPU frequency using sysinfo");

	let mut system = System::new();
	system.refresh_cpu_frequency();

	let frequency = system.cpus().first().unwrap().frequency();

	if !system.cpus().iter().all(|cpu| cpu.frequency() == frequency) {
		// Even if the CPU frequencies are not all equal, the
		// frequency of the "first" CPU is treated as "authoritative".
		eprintln!("CPU frequencies are not all equal");
	}

	// TODO: What can I do with the library's insistence of using u64 whereas I have to use u32?
	// Is try_into() enough?
	if frequency > 0 {
		Ok(frequency.try_into().unwrap())
	} else {
		Err(FrequencyDetectionFailed)
	}
}

#[cfg(test)]
mod tests {
	#[test]
	// derived from test_get_cpu_frequency_from_os() in src/arch/x86_64/mod.rs
	fn test_detect_freq_from_sysinfo() {
		let freq_res = crate::detect_freq_from_sysinfo();
		assert!(freq_res.is_ok());
		let freq = freq_res.unwrap();
		assert!(freq > 0);
		assert!(freq < 10000); // just like in the original test, more than 10Ghz is probably wrong
	}
}
