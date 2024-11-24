use std::{
	collections::HashMap,
	ffi::{CString, OsString},
	fmt, fs,
	fs::Permissions,
	os::unix::{ffi::OsStrExt, fs::PermissionsExt},
	path::PathBuf,
};

use tempfile::{Builder, TempDir};
use uuid::Uuid;

/// Creates a temporary directory.
pub fn create_temp_dir() -> TempDir {
	// TODO: Remove keep(true).
	let dir = Builder::new()
		.permissions(Permissions::from_mode(0o700))
		.prefix(&Uuid::new_v4().to_string())
		.keep(true)
		.suffix("-uhyve")
		.tempdir()
		.ok()
		.unwrap_or_else(|| panic!("The temporary directory could not be created."));

	let dir_permissions = dir.path().metadata().unwrap().permissions();
	assert!(!dir_permissions.readonly());

	dir
}

/// HashMap matching a path in the guest OS ([String]) a path in the host OS ([OsString]).
///
/// Using a list of parameters stored in a [Vec<String>], this function creates
/// a HashMap that can match a path on the host operating system given a path on
/// the guest operating system.
///
/// See [crate::hypercall::open] to see this in practice.
pub struct UhyveFileMap {
	files: HashMap<String, OsString>,
}

impl UhyveFileMap {
	/// Creates a UhyveFileMap.
	///
	/// * `parameters` - A list of parameters with the format `./host_path.txt:guest.txt`
	pub fn new(parameters: &Option<Vec<String>>) -> UhyveFileMap {
		if let Some(parameters) = parameters {
			UhyveFileMap {
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
			}
		} else {
			UhyveFileMap {
				files: Default::default(),
			}
		}
	}

	/// Separates a string of the format "./host_dir/host_path.txt:guest_path.txt"
	/// into a guest_path (String) and host_path (OsString) respectively.
	///
	/// `parameter` - A parameter of the format `./host_path.txt:guest.txt`.
	fn split_guest_and_host_path(parameter: &str) -> (String, OsString) {
		let mut partsiter = parameter.split(":");

		// Mind the order.
		// TODO: Do this work using clap.
		let host_path = OsString::from(partsiter.next().unwrap());
		let guest_path = partsiter.next().unwrap().to_owned();

		(guest_path, host_path)
	}

	/// Returns the host_path on the host filesystem given a requested guest_path, if it exists.
	///
	/// This function will look up the requested file in the UhyveFileMap and return
	/// the corresponding path. Internally, this function converts &OsString to OsString
	/// Otherwise, we would borrow UhyveFileMap in [crate::hypercall::open] as an
	/// immutable, when we may need a mutable borrow at a later point.
	///
	/// If the provided file is in a path containing directories, this function will
	/// try to look up whether a parent directory has been mapped. If this is
	/// the case, the child directories "in between" of the mapped directory and
	/// the requested file, as well as the file itself, will be added to the map.
	///
	/// * `guest_path` - The guest path. The file that the kernel is trying to open.
	pub fn get_host_path(&mut self, guest_path: &str) -> Option<OsString> {
		let host_path = self.files.get(guest_path).map(OsString::from);
		if host_path.is_some() {
			host_path
		} else {
			info!("Guest requested to open a path that was not mapped.");
			if self.files.is_empty() {
				info!("UhyveFileMap is empty, returning None...");
				return None;
			}

			let requested_guest_pathbuf = PathBuf::from(guest_path);
			if let Some(parent_of_guest_path) = requested_guest_pathbuf.parent() {
				info!("The file is in a child directory, searching for the directory...");
				let ancestors = parent_of_guest_path.ancestors();
				for searched_parent_guest in ancestors {
					let parent_host: Option<&OsString> =
						self.files.get(searched_parent_guest.to_str().unwrap());
					if let Some(parent_host) = parent_host {
						let mut host_path = PathBuf::from(parent_host);
						let mut new_guest_path = PathBuf::new();
						let guest_path_suffix = requested_guest_pathbuf
							.strip_prefix(searched_parent_guest)
							.unwrap();

						guest_path_suffix.components().for_each(|c| {
							host_path.push(c);
							new_guest_path.push(c);
							self.files.insert(
								new_guest_path.as_os_str().to_str().unwrap().to_owned(),
								host_path.as_os_str().to_os_string(),
							);
						});

						return host_path.into_os_string().into();
					}
				}
			}
			info!("The file is not in a child directory, returning None...");
			None
		}
	}

	pub fn append_file_and_return_cstring(
		&mut self,
		guest_path: &str,
		host_path: OsString,
	) -> CString {
		// TODO: Do we need to canonicalize the host_path?
		self.files
			.insert(String::from(guest_path), host_path.to_owned());

		CString::new(host_path.as_bytes()).unwrap()
	}
}

impl fmt::Debug for UhyveFileMap {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("UhyveFileMap")
			.field("files", &self.files)
			.finish()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_split_guest_and_host_path() {
		let host_guest_strings = vec![
			"./host_string.txt:guest_string.txt",
			"/home/user/host_string.txt:guest_string.md.txt",
			":guest_string.conf",
			":",
			"exists.txt:also_exists.txt:should_not_exist.txt",
		];

		// Mind the inverted order.
		let results = vec![
			(
				String::from("guest_string.txt"),
				OsString::from("./host_string.txt"),
			),
			(
				String::from("guest_string.md.txt"),
				OsString::from("/home/user/host_string.txt"),
			),
			(String::from("guest_string.conf"), OsString::from("")),
			(String::from(""), OsString::from("")),
			(
				String::from("also_exists.txt"),
				OsString::from("exists.txt"),
			),
		];

		for (i, host_and_guest_string) in host_guest_strings
			.into_iter()
			.map(UhyveFileMap::split_guest_and_host_path)
			.enumerate()
		{
			assert_eq!(host_and_guest_string, results[i]);
		}
	}

	#[test]
	fn test_uhyvefilemap() {
		// This entire section makes the test robust-ish enough, regardless of where
		// it is being run from. This presumes that the CARGO_MANIFEST_DIR is set
		// and absolute.
		//
		// Example: /home/user/uhyve
		let mut fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

		// Our files are in `$CARGO_MANIFEST_DIR/data/fixtures/fs`.
		//
		// If this is not true, this test will fail early so as to not confuse
		// the unlucky Uhyve developer.
		fixture_path.push("tests/data/fixtures/fs");
		assert!(fixture_path.is_dir());
		let path_prefix = fixture_path.to_str().unwrap().to_owned();

		// These are the desired host paths that we want the kernel to supposely use.
		//
		// The last case is a special case, the file's corresponding parameter
		// uses a symlink, which should be successfully resolved first.
		let map_results = [
			path_prefix.clone() + "/README.md",
			path_prefix.clone() + "/this_folder_exists",
			path_prefix.clone() + "/this_symlink_exists",
			path_prefix.clone() + "/this_symlink_is_dangling",
			path_prefix.clone() + "/this_file_does_not_exist",
			path_prefix.clone() + "/this_folder_exists/file_in_folder.txt",
		];

		// Each parameter has the format of host_path:guest_path
		let map_parameters = Some(vec![
			map_results[0].clone() + ":readme_file.md",
			map_results[1].clone() + ":guest_folder",
			map_results[2].clone() + ":guest_symlink",
			map_results[3].clone() + ":guest_dangling_symlink",
			map_results[4].clone() + ":guest_file",
			path_prefix.clone() + "/this_symlink_leads_to_a_file" + ":guest_file_symlink",
		]);

		let mut map = UhyveFileMap::new(&map_parameters);

		assert_eq!(
			map.get_host_path("readme_file.md").unwrap(),
			OsString::from(&map_results[0])
		);
		assert_eq!(
			map.get_host_path("guest_folder").unwrap(),
			OsString::from(&map_results[1])
		);
		assert_eq!(
			map.get_host_path("guest_symlink").unwrap(),
			OsString::from(&map_results[2])
		);
		assert_eq!(
			map.get_host_path("guest_dangling_symlink").unwrap(),
			OsString::from(&map_results[3])
		);
		assert_eq!(
			map.get_host_path("guest_file").unwrap(),
			OsString::from(&map_results[4])
		);
		assert_eq!(
			map.get_host_path("guest_file_symlink").unwrap(),
			OsString::from(&map_results[5])
		);

		assert!(map.get_host_path("this_file_is_not_mapped").is_none());
	}

	#[test]
	fn test_uhyvefilemap_folder() {
		// See `test_uhyvefilemap()`
		let mut fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
		fixture_path.push("tests/data/fixtures/fs");
		assert!(fixture_path.is_dir());

		// Tests successful directory traversal starting from file in child
		// directory of a mapped directory.
		let guest_path_map = PathBuf::from("this_folder_exists");
		let mut host_path_map = fixture_path.clone();
		host_path_map.push("this_folder_exists");

		let mut target_guest_path =
			PathBuf::from("this_folder_exists/folder_in_folder/file_in_second_folder.txt");
		let mut target_host_path = fixture_path;
		target_host_path.push(target_guest_path.clone());

		let uhyvefilemap_params = vec![format!(
			"{}:{}",
			host_path_map.to_str().unwrap(),
			guest_path_map.to_str().unwrap()
		)];
		let mut map = UhyveFileMap::new(&uhyvefilemap_params.into());

		let mut found_host_path = map.get_host_path(target_guest_path.clone().to_str().unwrap());

		assert_eq!(
			found_host_path.unwrap(),
			target_host_path.as_os_str().to_str().unwrap()
		);

		// Tests successful directory traversal of the child directory.
		// The pop() just removes the text file.
		// guest_path.pop();
		target_host_path.pop();
		target_guest_path.pop();

		found_host_path = map.get_host_path(target_guest_path.to_str().unwrap());
		assert_eq!(
			found_host_path.unwrap(),
			target_host_path.as_os_str().to_str().unwrap()
		);

		// Tests directory traversal with no maps
		map = UhyveFileMap::new(&None);
		found_host_path = map.get_host_path(target_guest_path.to_str().unwrap());
		assert!(found_host_path.is_none());
	}
}
