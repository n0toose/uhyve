use std::{collections::HashMap, fs, path::PathBuf, str::FromStr, vec::Vec};

pub struct UhyveFileParameters {
	files: HashMap<PathBuf, PathBuf>,
	empty_files: HashMap<PathBuf, PathBuf>,
	enabled: bool,
}

impl UhyveFileParameters {
	pub fn new() -> UhyveFileParameters {
		// The first PathBuf corresponds to the guest path (our key),
		// the second one to the host's.
		let files: HashMap<PathBuf, PathBuf> = HashMap::new();
		let empty_files: HashMap<PathBuf, PathBuf> = HashMap::new();

		Self {
			files,
			empty_files,
			enabled: false,
		}
	}

	pub fn populate(&mut self, parameters: Vec<String>) -> () {
		// This is what our approach looks like:
		// 1. fs::canonicalize, check if it exists or not
		// 2. Check if is is_file.
		// 3. If neither:
		//   - Check if the path is a valid location.
		//   - If it isn't, consider it as an empty file.
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
					self.empty_files.insert(guest_path, host_path);
				}
			}
		}
	}

	fn split_host_and_guest_path(entry: &String) -> (PathBuf, PathBuf) {
		let parts: Vec<&str> = entry.split(":").collect();
		// Deal with the host OS's path first.

		let host_path = PathBuf::from_str(parts.clone().get(0).unwrap()).unwrap();

		// Uses /root + the name of the file (or the symbolic link) on the host OS
		// if no specific location has been supplied.
		let guest_path = parts.get(1).map(|s| PathBuf::from(s)).unwrap_or_else(|| {
			let mut new_path = PathBuf::new();
			new_path.push("/root");
			new_path.push(host_path.file_name().unwrap());

			new_path
		});

		(guest_path, host_path)
	}
}
