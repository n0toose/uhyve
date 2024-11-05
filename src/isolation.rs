use std::{
	collections::HashMap,
	ffi::{CString, OsString},
	fs,
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
	/// * `parameters` - A list of parameters with the format `./host_path.txt:guest.txt`
	pub fn new(parameters: &[String]) -> Option<UhyveFileMap> {
		// The CString is the guest path, the OsString is the host path.
		let mut files: HashMap<CString, OsString> = HashMap::new();

		// TODO: Introduce additional option for fully disabling filesystem access.
		// TODO: Introduce additional option that allows storing non-whitelisted files in `/tmp`.
		for parameter in parameters.iter() {
			// fs::canonicalize resolves the absolute path. It also resolves symlinks,
			// so we don't have to check for that edge case later on.
			//
			// Keep in mind that the order of host_path and guest_path has been swapped,
			// in comparison to split_guest_and_host_path, so as to make key-value
			// lookups possible. (See: `src/hypercall.rs`)
			let (guest_path, host_path) = Self::split_guest_and_host_path(parameter);
			let canonicalized_path = fs::canonicalize(&host_path);
			match canonicalized_path {
				Ok(p) => {
					files.insert(guest_path, p.into_os_string());
				}
				Err(_e) => {
					// If resolving the path is not possible (i.e. it does not exist),
					// store it anyway. If the kernel opens this guest_path, this will
					// create a new file in the host operating system.
					files.insert(guest_path, host_path);
				}
			}
		}

		Some(UhyveFileMap { files })
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
