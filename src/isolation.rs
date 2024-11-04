use std::{collections::HashMap, fs, path::PathBuf, ffi::OsString, str::FromStr, vec::Vec};

pub struct UhyveFileParameters {
	files: HashMap<OsString, PathBuf>,
	enabled: bool,
}

impl UhyveFileParameters {
	pub fn new() -> UhyveFileParameters {
		// The first PathBuf corresponds to the guest path (our key),
		// the second one to the host's.
		let files: HashMap<OsString, PathBuf> = HashMap::new();

		Self {
			files,
			enabled: false,
		}
	}

	pub fn populate(&mut self, parameters: Vec<String>) -> () {
		if parameters.is_empty() {
			return;
		} else {
			self.enabled = true;
		}

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
			let canonicalized_path = fs::canonicalize(host_path.clone());
			match canonicalized_path {
				Ok(p) => {
					self.files.insert(guest_path, p);
				}
				Err(_e) => {
					// Store path in hash_map's for missing paths.
					// TODO: If there are empty files, do we explicitly only use them?
					// TODO: Do we discard other files that may be created by the kernel, or do we store them in a tmpfs?
					self.files.insert(guest_path, host_path);
				}
			}
		}
	}

	fn split_host_and_guest_path(entry: &String) -> (OsString, PathBuf) {
		let parts: Vec<&str> = entry.split(":").collect();
		// Deal with the host OS's path first.

		let host_path = PathBuf::from_str(parts.clone().get(0).unwrap()).unwrap();

		// Uses /root + the name of the file (or the symbolic link) on the host OS
		// if no specific location has been supplied.
		let guest_path = parts.get(1).map(|s| OsString::from(s)).unwrap_or_else(|| {
			let mut new_path = OsString::new();
			new_path.push("/root/");
			new_path.push(host_path.file_name().unwrap());

			new_path
		});

		(guest_path, host_path)
	}

	pub fn get_paths(&self) -> HashMap<OsString, PathBuf> {
		return self.files.clone()
	}
}
