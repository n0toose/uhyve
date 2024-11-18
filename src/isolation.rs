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

/// Creates a temporary directory. This is currently being done in a separate
/// function because of the complexity level and to allow for more granular
/// OS-specific approaches later on.
///
/// * `tempdir` - Custom temporary directory.
/// * `test` - Creates a test folder with the prefix "testprefix". Files will be retained.
pub fn create_temp_dir(tempdir: Option<PathBuf>, test: bool) -> Option<TempDir> {
	// TODO: Move this into a separate function
	if test {
		return Builder::new()
			.permissions(Permissions::from_mode(0o700))
			.prefix("testprefix")
			.rand_bytes(0)
			.keep(true)
			.tempdir_in(tempdir.unwrap())
			.ok();
	}

	if let Some(tempdir) = tempdir {
		let tempdir_path = PathBuf::from(&tempdir);

		match &tempdir_path.metadata() {
			Ok(metadata) => {
				// If the path exists, ensure that we can actually use it.
				assert!(metadata.is_dir() && !metadata.permissions().readonly());

				Builder::new()
					.permissions(Permissions::from_mode(0o700))
					.prefix(&Uuid::new_v4().to_string())
					.tempdir_in(tempdir_path)
					.ok()
			}
			Err(e) => {
				panic!(
					"The directory {:#?} does not exist or cannot be accessed: {}",
					tempdir_path, e
				);
			}
		}
	} else {
		Builder::new()
			.permissions(Permissions::from_mode(0o700))
			.prefix(&Uuid::new_v4().to_string())
			.tempdir()
			.ok()
	}
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
	/// the corresponding path. Used in [`Hyper`] Internally, this function converts
	/// &OsString to OsString. Otherwise, we would borrow UhyveFileMap in
	/// [crate::hypercall::open] as an immutable, when we may need a mutable borrow
	/// at a later point.
	///
	/// `guest_path` - The guest path. The file that the kernel is trying to open.
	pub fn get_host_path(&mut self, guest_path: &str) -> Option<OsString> {
		self.files.get(guest_path).map(OsString::from)
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
	use std::panic;

	use super::*;

	#[test]
	fn test_create_temp_dir() {
		// test is never true, as its correctness is tested in fs-test and by upstream
		// other types of runtime weirdness should be checked using assertions
		let mut temp_dir = create_temp_dir(None, false).unwrap();
		assert!(temp_dir.path().exists());
		temp_dir = create_temp_dir(Some(PathBuf::from("/tmp")), false).unwrap();
		assert!(temp_dir.path().exists());

		// This suppresses the panic.
		// See: https://doc.rust-lang.org/std/panic/fn.set_hook.html
		panic::set_hook(Box::new(|_| {}));
		let result = panic::catch_unwind(|| {
			create_temp_dir(Some(PathBuf::from("/this/should/not/exist")), false)
		});
		assert!(result.is_err());
	}

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
		let map_results = vec![
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
}
