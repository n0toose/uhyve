use std::{collections::HashMap, ffi::OsString, fs, path::PathBuf, str::FromStr, vec::Vec};

pub struct UhyveFileMap {
	files: HashMap<OsString, PathBuf>,
}

impl UhyveFileMap {
	pub fn new(parameters: &[String]) -> Option<UhyveFileMap> {
		// The first PathBuf corresponds to the guest path (our key),
		// the second one to the host's.
		let mut files: HashMap<OsString, PathBuf> = HashMap::new();
		
		if parameters.is_empty() {
			return None;
		}

		// TODO: maybe use a split for this instead
		for parameter in parameters.iter() {
			// fs::canonicalize resolves the absolute path. It also resolves symlinks,
			// so we don't have to check for that edge case later on.
			//
			// This part effectively adds all paths and categorizes them,
			// using the guest OS path as a key. HashMaps are not expensive.
			//
			// Keep in mind that the order of host_path and guest_path has been swapped,
			// in comparison to split_host_and_guest_path, so as to make lookups in hypercall.rs
			// easier.
			let (guest_path, host_path) = Self::split_host_and_guest_path(parameter);
			let canonicalized_path = fs::canonicalize(&host_path);
			match canonicalized_path {
				Ok(p) => {
					files.insert(guest_path, p);
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

	fn split_host_and_guest_path(entry: &String) -> (OsString, PathBuf) {
		let mut partsiter = entry.split(":");

		let guest_path = OsString::from(partsiter.next().unwrap());
		let host_path = PathBuf::from(partsiter.next().unwrap());

		(guest_path, host_path)
	}

	pub fn get_paths(&self) -> &HashMap<OsString, PathBuf> {
		&self.files
	}
}
