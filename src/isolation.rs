use std::{
	collections::HashMap,
	ffi::{CString, OsString},
	fs,
};

pub struct UhyveFileMap {
	// CString: guest, OsString: host
	files: HashMap<CString, OsString>,
}

impl UhyveFileMap {
	pub fn new(parameters: &Option<&[String]>) -> Option<UhyveFileMap> {
		// The first component corresponds to the guest path (our key),
		// the second one to the host's.
		let mut files: HashMap<CString, OsString> = HashMap::new();

		// TODO: Introduce additional option for fully disabling filesystem access.
		if parameters.is_none() {
			println!("No --mount parameters provided. The hypervisor will provide full host filesystem access to the kernel!");
			return None;
		}

		for parameter in parameters.unwrap().iter() {
			// fs::canonicalize resolves the absolute path. It also resolves symlinks,
			// so we don't have to check for that edge case later on.
			//
			// This part effectively adds all paths and categorizes them,
			// using the guest OS path as a key. HashMaps are not expensive.
			//
			// Keep in mind that the order of host_path and guest_path has been swapped,
			// in comparison to split_host_and_guest_path, so as to make lookups in hypercall.rs
			// easier.
			let (guest_path, host_path) = Self::split_guest_and_host_path(parameter);
			let canonicalized_path = fs::canonicalize(&host_path);
			match canonicalized_path {
				Ok(p) => {
					files.insert(guest_path, p.into_os_string());
				}
				Err(_e) => {
					// If resolving the path is not possible,
					// let's just store it anyway for now.
					files.insert(guest_path, host_path);
				}
			}
		}

		return Some(UhyveFileMap { files });
	}

	fn split_guest_and_host_path(entry: &String) -> (CString, OsString) {
		let mut partsiter = entry.split(":");

		let host_path = OsString::from(partsiter.next().unwrap());
		let guest_path = CString::new(partsiter.next().unwrap()).unwrap();

		(guest_path, host_path)
	}

	pub fn get_paths(&self) -> &HashMap<CString, OsString> {
		&self.files
	}
}
