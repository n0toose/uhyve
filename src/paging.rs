//! General paging related code
use thiserror::Error;
use uhyve_interface::GuestPhysAddr;
// TODO: Clean this up.
use x86_64::{
	structures::paging::{Page, PageTable, PageTableFlags, Size2MiB},
	PhysAddr,
};

use crate::consts::*;

#[derive(Error, Debug)]
pub enum PagetableError {
	#[error("The accessed virtual address is not mapped")]
	InvalidAddress,
}

#[derive(Clone, Copy, Debug)]
pub struct UhyvePageTable {
	pub BOOT_GDT: GuestPhysAddr,
	pub BOOT_PML4: GuestPhysAddr,
	pub BOOT_PGT: GuestPhysAddr,
	pub BOOT_PDPTE: GuestPhysAddr,
	pub BOOT_PDE: GuestPhysAddr,
	pub BOOT_INFO_ADDR: GuestPhysAddr,
}

// TODO: Get this x86_64 code out of here, only here for convenience.
// TODO: Check if the values of an object are set or not.
impl UhyvePageTable {
	pub fn new(guest_address: GuestPhysAddr) -> UhyvePageTable {
		let memory_start = guest_address.as_u64();
		let BOOT_GDT = GuestPhysAddr::new(memory_start + GDT_OFFSET);
		let BOOT_PML4 = GuestPhysAddr::new(memory_start + PML4_OFFSET);
		let BOOT_PGT = GuestPhysAddr::new(memory_start + PGT_OFFSET);
		let BOOT_PDPTE = GuestPhysAddr::new(memory_start + PDPTE_OFFSET);
		let BOOT_PDE = GuestPhysAddr::new(memory_start + PDE_OFFSET);
		let BOOT_INFO_ADDR = GuestPhysAddr::new(INFO_ADDR_OFFSET);

		UhyvePageTable {
			BOOT_GDT,
			BOOT_PML4,
			BOOT_PGT,
			BOOT_PDPTE,
			BOOT_PDE,
			BOOT_INFO_ADDR,
		}
	}

	/// Creates the pagetables and the GDT in the guest memory space.
	///
	/// The memory slice must be larger than [`MIN_PHYSMEM_SIZE`].
	/// Also, the memory `mem` needs to be zeroed for [`PAGE_SIZE`] bytes at the
	/// offsets [`BOOT_PML4`] and [`BOOT_PDPTE`], otherwise the integrity of the
	/// pagetables and thus the integrity of the guest's memory is not ensured
	pub fn initialize_pagetables(&self, mem: &mut [u8]) {
		assert!(mem.len() >= self.get_min_physmem_size());
		let mem_addr = std::ptr::addr_of_mut!(mem[0]);

		let (gdt_entry, pml4, pdpte, pde);
		// Safety:
		// We only operate in `mem`, which is plain bytes and we have ownership of
		// these and it is asserted to be large enough.
		unsafe {
			gdt_entry = mem_addr
				.add(self.BOOT_GDT.as_u64() as usize)
				.cast::<[u64; 3]>()
				.as_mut()
				.unwrap();

			pml4 = mem_addr
				.add(self.BOOT_PML4.as_u64() as usize)
				.cast::<PageTable>()
				.as_mut()
				.unwrap();
			pdpte = mem_addr
				.add(self.BOOT_PDPTE.as_u64() as usize)
				.cast::<PageTable>()
				.as_mut()
				.unwrap();
			pde = mem_addr
				.add(self.BOOT_PDE.as_u64() as usize)
				.cast::<PageTable>()
				.as_mut()
				.unwrap();

			/* For simplicity we currently use 2MB pages and only a single
			PML4/PDPTE/PDE. */

			// per default is the memory zeroed, which we allocate by the system
			// call mmap, so the following is not necessary:
			/*libc::memset(pml4 as *mut _ as *mut libc::c_void, 0x00, PAGE_SIZE);
			libc::memset(pdpte as *mut _ as *mut libc::c_void, 0x00, PAGE_SIZE);
			libc::memset(pde as *mut _ as *mut libc::c_void, 0x00, PAGE_SIZE);*/
		}
		// initialize GDT
		gdt_entry[BOOT_GDT_NULL] = 0;
		gdt_entry[BOOT_GDT_CODE] = self.create_gdt_entry(0xA09B, 0, 0xFFFFF);
		gdt_entry[BOOT_GDT_DATA] = self.create_gdt_entry(0xC093, 0, 0xFFFFF);

		pml4[0].set_addr(
			self.BOOT_PDPTE,
			PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
		);
		pml4[511].set_addr(
			self.BOOT_PML4,
			PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
		);
		pdpte[0].set_addr(
			self.BOOT_PDE,
			PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
		);

		for i in 0..512 {
			let addr = PhysAddr::new(i as u64 * Page::<Size2MiB>::SIZE);
			pde[i].set_addr(
				addr,
				PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::HUGE_PAGE,
			);
		}
	}

	pub fn init_guest_mem(&self, mem: &mut [u8]) {
		// TODO: we should maybe return an error on failure (e.g., the memory is too small)
		self.initialize_pagetables(mem);
	}

	pub fn get_min_physmem_size(&self) -> usize {
		self.BOOT_PDE.as_u64() as usize + 0x1000
	}

	// Constructor for a conventional segment GDT (or LDT) entry
	pub fn create_gdt_entry(&self, flags: u64, base: u64, limit: u64) -> u64 {
		((base & 0xff000000u64) << (56 - 24))
			| ((flags & 0x0000f0ffu64) << 40)
			| ((limit & 0x000f0000u64) << (48 - 16))
			| ((base & 0x00ffffffu64) << 16)
			| (limit & 0x0000ffffu64)
	}
}
