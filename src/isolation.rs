use std::{
	collections::HashMap,
	ffi::{CString, OsString},
	fs,
	path::PathBuf,
};

/// HashMap matching a path in the guest OS (`CString`) a path in the host OS (`OsString`).
pub struct UhyveFileMap {
	files: HashMap<CString, OsString>,
}

impl UhyveFileMap {
	/// Creates a UhyveFileMap.
	///
	/// Using a list of parameters stored in a Vec<String>, this function creates
	/// a HashMap that can match a path on the host operating system given a path on
	/// the guest operating system.
	///
	/// See [crate::hypercall::open] to see this in practice.
	///
	/// * `parameters` - A list of parameters with the format `./host_path.txt:guest.txt`
	pub fn new(parameters: &[String]) -> Option<UhyveFileMap> {
		Some(UhyveFileMap {
			files: parameters
				.iter()
				.map(String::as_str)
				.map(Self::split_guest_and_host_path)
				.map(|(guest_path, host_path)| {
					(
						guest_path,
						fs::canonicalize(&host_path).map_or(host_path, PathBuf::into_os_string),
					)
				})
				.collect(),
		})
	}

	/// Separates a string of the format "./host_dir/host_path.txt:guest_path.txt"
	/// into a guest_path (CString) and host_path (OsString) respectively.
	///
	/// Keep in mind that the order of the parameters is the inverse of the one
	/// in the actual HashMap itself, as we want to use the guest_path as a key
	/// to look up the respective host_path, as well as provide an intuitive
	/// interface reminiscent of other VMMs like Docker's.
	///
	/// `parameter` - A parameter of the format `./host_path.txt:guest.txt`.
	fn split_guest_and_host_path(parameter: &str) -> (CString, OsString) {
		let mut partsiter = parameter.split(":");

		// Mind the order.
		// TODO: Do this work using clap.
		let host_path = OsString::from(partsiter.next().unwrap());
		let guest_path = CString::new(partsiter.next().unwrap()).unwrap();

		(guest_path, host_path)
	}

	/// Returns a reference to the stored HashMap.
	///
	/// This function is commonly used with get_key_value, using a CString
	/// (that is read from a `const char*` in an `open()` call) as a key.
	pub fn get_paths(&self) -> &HashMap<CString, OsString> {
		&self.files
	}
}
