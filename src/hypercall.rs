use std::{
	ffi::{CStr, CString, OsStr},
	io::{self, Error, ErrorKind, Write},
	os::unix::ffi::OsStrExt,
};

use uhyve_interface::{parameters::*, GuestPhysAddr, Hypercall, HypercallAddress, MAX_ARGC_ENVC};

use crate::{
	consts::BOOT_PML4,
	isolation::UhyveFileMap,
	mem::{MemoryError, MmapMemory},
	virt_to_phys,
};

/// `addr` is the address of the hypercall parameter in the guest's memory space. `data` is the
/// parameter that was send to that address by the guest.
///
/// # Safety
///
/// - The return value is only valid, as long as the guest is halted.
/// - This fn must not be called multiple times on the same data, to avoid creating mutable aliasing.
pub unsafe fn address_to_hypercall(
	mem: &MmapMemory,
	addr: u16,
	data: GuestPhysAddr,
) -> Option<Hypercall<'_>> {
	if let Ok(hypercall_port) = HypercallAddress::try_from(addr) {
		Some(match hypercall_port {
			HypercallAddress::FileClose => {
				let sysclose = mem.get_ref_mut::<CloseParams>(data).unwrap();
				// let sysclose = unsafe { &mut *(self.host_address(data) as *mut CloseParams) };
				Hypercall::FileClose(sysclose)
			}
			HypercallAddress::FileLseek => {
				let syslseek = mem.get_ref_mut::<LseekParams>(data).unwrap();
				Hypercall::FileLseek(syslseek)
			}
			HypercallAddress::FileOpen => {
				let sysopen = mem.get_ref_mut::<OpenParams>(data).unwrap();
				Hypercall::FileOpen(sysopen)
			}
			HypercallAddress::FileRead => {
				let sysread = mem.get_ref_mut::<ReadParams>(data).unwrap();
				Hypercall::FileRead(sysread)
			}
			HypercallAddress::FileWrite => {
				let syswrite = mem.get_ref_mut(data).unwrap();
				Hypercall::FileWrite(syswrite)
			}
			HypercallAddress::FileUnlink => {
				let sysunlink = mem.get_ref_mut(data).unwrap();
				Hypercall::FileUnlink(sysunlink)
			}
			HypercallAddress::Exit => {
				let sysexit = mem.get_ref_mut(data).unwrap();
				Hypercall::Exit(sysexit)
			}
			HypercallAddress::Cmdsize => {
				let syssize = mem.get_ref_mut(data).unwrap();
				Hypercall::Cmdsize(syssize)
			}
			HypercallAddress::Cmdval => {
				let syscmdval = mem.get_ref_mut(data).unwrap();
				Hypercall::Cmdval(syscmdval)
			}
			HypercallAddress::Uart => Hypercall::SerialWriteByte(data.as_u64() as u8),
			HypercallAddress::SerialBufferWrite => {
				let sysserialwrite = mem.get_ref_mut(data).unwrap();
				Hypercall::SerialWriteBuffer(sysserialwrite)
			}
			_ => unimplemented!(),
		})
	} else {
		None
	}
}

/// unlink deletes a name from the filesystem. This is used to handle `unlink` syscalls from the guest.
/// TODO: UNSAFE AS *%@#. It has to be checked that the VM is allowed to unlink that file!
pub fn unlink(mem: &MmapMemory, sysunlink: &mut UnlinkParams) {
	unsafe {
		sysunlink.ret = libc::unlink(mem.host_address(sysunlink.name).unwrap() as *const i8);
	}
}

/// Handles an open syscall by opening a file on the host.
pub fn open(mem: &MmapMemory, sysopen: &mut OpenParams, file_map: &Option<UhyveFileMap>) {
	// TODO: We could keep track of the file descriptors internally, in case the kernel doesn't close them.
	let requested_path = mem.host_address(sysopen.name).unwrap() as *const i8;

	// If the file_map doesn't exist, full host filesystem access will be provided.
	if let Some(file_map) = file_map {
		// Rust deals in UTF-8. C doesn't provide such a guarantee.
		// In that case, converting a CStr to str will return a Utf8Error.
		//
		// See: https://nrc.github.io/big-book-ffi/reference/strings.html
		let guest_path = unsafe { CStr::from_ptr(requested_path) }.to_str();

		if let Ok(guest_path) = guest_path {
			let paths = file_map;
			let host_path_option = paths.get_paths().get_key_value(guest_path);

			if let Some((_guest_path, host_path)) = host_path_option {
				// This variable has to exist, as pointers don't have a lifetime
				// and appending .as_ptr() would lead to the string getting
				// immediately deallocated after the statement. Nothing is
				// referencing it as far as the type system is concerned".
				//
				// This is also why we can't just have one unsafe block and
				// one path variable, otherwise we'll get a use after free.
				let host_path_c_string = CString::new(host_path.as_bytes()).unwrap();
				let new_host_path = host_path_c_string.as_c_str().as_ptr();

				unsafe {
					sysopen.ret = libc::open(new_host_path, sysopen.flags, sysopen.mode);
				}
			} else {
				error!("The kernel requested to open() a non-whitelisted path. Rejecting...");
				sysopen.ret = -1;
			}
		} else {
			error!("The kernel requested to open() a path that is not valid UTF-8. Rejecting...");
			sysopen.ret = -1;
		}
	} else {
		unsafe {
			sysopen.ret = libc::open(requested_path, sysopen.flags, sysopen.mode);
		}
	}
}

/// Handles an close syscall by closing the file on the host.
pub fn close(sysclose: &mut CloseParams) {
	unsafe {
		sysclose.ret = libc::close(sysclose.fd);
	}
}

/// Handles an read syscall on the host.
pub fn read(mem: &MmapMemory, sysread: &mut ReadParams) {
	unsafe {
		let bytes_read = libc::read(
			sysread.fd,
			mem.host_address(virt_to_phys(sysread.buf, mem, BOOT_PML4).unwrap())
				.unwrap() as *mut libc::c_void,
			sysread.len,
		);
		if bytes_read >= 0 {
			sysread.ret = bytes_read;
		} else {
			sysread.ret = -1;
		}
	}
}

/// Handles an write syscall on the host.
pub fn write(mem: &MmapMemory, syswrite: &WriteParams) -> io::Result<()> {
	let mut bytes_written: usize = 0;
	while bytes_written != syswrite.len {
		unsafe {
			let step = libc::write(
				syswrite.fd,
				mem.host_address(
					virt_to_phys(syswrite.buf + bytes_written as u64, mem, BOOT_PML4).unwrap(),
				)
				.map_err(|e| match e {
					MemoryError::BoundsViolation => {
						unreachable!("Bounds violation after host_address function")
					}
					MemoryError::WrongMemoryError => {
						Error::new(ErrorKind::AddrNotAvailable, e.to_string())
					}
				})? as *const libc::c_void,
				syswrite.len - bytes_written,
			);
			if step >= 0 {
				bytes_written += step as usize;
			} else {
				return Err(io::Error::last_os_error());
			}
		}
	}

	Ok(())
}

/// Handles an write syscall on the host.
pub fn lseek(syslseek: &mut LseekParams) {
	unsafe {
		syslseek.offset =
			libc::lseek(syslseek.fd, syslseek.offset as i64, syslseek.whence) as isize;
	}
}

/// Handles an UART syscall by writing to stdout.
pub fn uart(buf: &[u8]) -> io::Result<()> {
	io::stdout().write_all(buf)
}

/// Handles a UART syscall by contructing a buffer from parameter
pub fn uart_buffer(sysuart: &SerialWriteBufferParams, mem: &MmapMemory) {
	let buf = unsafe {
		mem.slice_at(sysuart.buf, sysuart.len)
			.expect("Systemcall parameters for SerialWriteBuffer are invalid")
	};
	io::stdout().write_all(buf).unwrap()
}

/// Copies the arguments of the application into the VM's memory to the destinations specified in `syscmdval`.
pub fn copy_argv(path: &OsStr, argv: &[String], syscmdval: &CmdvalParams, mem: &MmapMemory) {
	// copy kernel path as first argument
	let argvp = mem
		.host_address(syscmdval.argv)
		.expect("Systemcall parameters for Cmdval are invalid") as *const GuestPhysAddr;
	let arg_addrs = unsafe { std::slice::from_raw_parts(argvp, argv.len() + 1) };

	{
		let len = path.len();
		// Safety: we drop path_dest before anything else is done with mem
		let path_dest = unsafe {
			mem.slice_at_mut(arg_addrs[0], len + 1)
				.expect("Systemcall parameters for Cmdval are invalid")
		};

		path_dest[0..len].copy_from_slice(path.as_bytes());
		path_dest[len] = 0; // argv strings are zero terminated
	}

	// Copy the application arguments into the vm memory
	for (counter, argument) in argv.iter().enumerate() {
		let len = argument.as_bytes().len();
		let arg_dest = unsafe {
			mem.slice_at_mut(arg_addrs[counter], len + 1)
				.expect("Systemcall parameters for Cmdval are invalid")
		};
		arg_dest[0..len].copy_from_slice(argument.as_bytes());
		arg_dest[len] = 0;
	}
}

/// Copies the environment variables into the VM's memory to the destinations specified in `syscmdval`.
pub fn copy_env(syscmdval: &CmdvalParams, mem: &MmapMemory) {
	let env_len = std::env::vars_os().count();
	let envp = mem
		.host_address(syscmdval.envp)
		.expect("Systemcall parameters for Cmdval are invalid") as *const GuestPhysAddr;
	let env_addrs = unsafe { std::slice::from_raw_parts(envp, env_len) };

	// Copy the environment variables into the vm memory
	for (counter, (key, value)) in std::env::vars_os().enumerate() {
		if counter >= MAX_ARGC_ENVC.try_into().unwrap() {
			warn!("Environment is larger than the maximum that can be copied to the VM. Remaining environment is ignored");
			break;
		}

		let len = key.len() + value.len() + 1;
		let env_dest = unsafe {
			mem.slice_at_mut(env_addrs[counter], len + 1)
				.expect("Systemcall parameters for Cmdval are invalid")
		};
		env_dest[0..key.len()].copy_from_slice(key.as_bytes());
		env_dest[key.len()] = b'=';
		env_dest[key.len() + 1..len].copy_from_slice(value.as_bytes());
		env_dest[len] = 0;
	}
}
