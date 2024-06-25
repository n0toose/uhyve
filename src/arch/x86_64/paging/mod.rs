use uhyve_interface::GuestPhysAddr;
use x86_64::{
	structures::paging::{Page, PageTable, PageTableFlags, Size2MiB},
	PhysAddr,
};

use crate::consts::*;

// Constructor for a conventional segment GDT (or LDT) entry
pub fn create_gdt_entry(flags: u64, base: u64, limit: u64) -> u64 {
	((base & 0xff000000u64) << (56 - 24))
		| ((flags & 0x0000f0ffu64) << 40)
		| ((limit & 0x000f0000u64) << (48 - 16))
		| ((base & 0x00ffffffu64) << 16)
		| (limit & 0x0000ffffu64)
}

/// Creates the pagetables and the GDT in the guest memory space.
///
/// The memory slice must be larger than [`MIN_PHYSMEM_SIZE`].
/// Also, the memory `mem` needs to be zeroed for [`PAGE_SIZE`] bytes at the
/// offsets [`BOOT_PML4`] and [`BOOT_PDPTE`], otherwise the integrity of the
/// pagetables and thus the integrity of the guest's memory is not ensured
/// `mem` and `GuestPhysAddr` must be 2MiB page aligned.
pub fn initialize_pagetables(mem: &mut [u8], guest_address: GuestPhysAddr) {
	assert!(mem.len() >= MIN_PHYSMEM_SIZE);
	let mem_addr = std::ptr::addr_of_mut!(mem[0]);

	let (gdt_entry, pml4, pdpte, pde);
	// Safety:
	// We only operate in `mem`, which is plain bytes and we have ownership of
	// these and it is asserted to be large enough.
	unsafe {
		gdt_entry = mem_addr
			.add(GDT_OFFSET as usize)
			.cast::<[u64; 3]>()
			.as_mut()
			.unwrap();

		pml4 = mem_addr
			.add(PML4_OFFSET as usize)
			.cast::<PageTable>()
			.as_mut()
			.unwrap();
		pdpte = mem_addr
			.add(PDPTE_OFFSET as usize)
			.cast::<PageTable>()
			.as_mut()
			.unwrap();
		pde = mem_addr
			.add(PDE_OFFSET as usize)
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
	gdt_entry[BOOT_GDT_CODE] = create_gdt_entry(0xA09B, 0, 0xFFFFF);
	gdt_entry[BOOT_GDT_DATA] = create_gdt_entry(0xC093, 0, 0xFFFFF);

	pml4[0].set_addr(
		(guest_address + PDPTE_OFFSET).into(),
		PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
	);
	pml4[511].set_addr(
		(guest_address + PML4_OFFSET).into(),
		PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
	);
	pdpte[0].set_addr(
		(guest_address + PDE_OFFSET).into(),
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

#[allow(dead_code)]
/// Helper fn for debugging pagetables
fn pretty_print_pagetable(pt: &PageTable) {
	println!("Idx       Address          Idx       Address          Idx       Address          Idx       Address      ");
	println!("--------------------------------------------------------------------------------------------------------");
	for i in (0..512).step_by(4) {
		println!(
			"{:3}: {:#18x},   {:3}: {:#18x},   {:3}: {:#18x},   {:3}: {:#18x}",
			i,
			pt[i].addr(),
			i + 1,
			pt[i + 1].addr(),
			i + 2,
			pt[i + 2].addr(),
			i + 3,
			pt[i + 3].addr()
		);
	}
	println!("--------------------------------------------------------------------------------------------------------");
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		consts::{GDT_OFFSET, PDE_OFFSET, PDPTE_OFFSET, PML4_OFFSET},
		mem::HugePageAlignedMem,
	};

	#[test]
	fn test_pagetable_initialization() {
		let _ = env_logger::builder()
			.filter(None, log::LevelFilter::Debug)
			.is_test(true)
			.try_init();
		let guest_address = GuestPhysAddr::new(0x20000);

		let aligned_mem = HugePageAlignedMem::<MIN_PHYSMEM_SIZE>::new();
		// This will return a pagetable setup that we will check.
		initialize_pagetables((aligned_mem.mem).try_into().unwrap(), guest_address);

		// Check PDPTE address
		let addr_pdpte = GuestPhysAddr::new(u64::from_le_bytes(
			aligned_mem.mem[(PML4_OFFSET as usize)..(PML4_OFFSET as usize + 8)]
				.try_into()
				.unwrap(),
		));
		assert_eq!(
			addr_pdpte - guest_address,
			PDPTE_OFFSET | (PageTableFlags::PRESENT | PageTableFlags::WRITABLE).bits()
		);

		// Check PDE
		let addr_pde = GuestPhysAddr::new(u64::from_le_bytes(
			aligned_mem.mem[(PDPTE_OFFSET as usize)..(PDPTE_OFFSET as usize + 8)]
				.try_into()
				.unwrap(),
		));
		assert_eq!(
			addr_pde - guest_address,
			PDE_OFFSET | (PageTableFlags::PRESENT | PageTableFlags::WRITABLE).bits()
		);

		// Check PDE's pagetable bits
		for i in (0..4096).step_by(8) {
			let pde_addr = (PDE_OFFSET) as usize + i;
			let entry = u64::from_le_bytes(
				aligned_mem.mem[pde_addr..(pde_addr + 8)]
					.try_into()
					.unwrap(),
			);
			assert!(
				PageTableFlags::from_bits_truncate(entry)
					.difference(
						PageTableFlags::PRESENT
							| PageTableFlags::WRITABLE
							| PageTableFlags::HUGE_PAGE
					)
					.is_empty(),
				"Pagetable bits at {pde_addr:#x} are incorrect"
			)
		}

		// Test GDT
		let gdt_results = [0x0, 0xAF9B000000FFFF, 0xCF93000000FFFF];
		for (i, res) in gdt_results.iter().enumerate() {
			let gdt_addr = GDT_OFFSET as usize + i * 8;
			let gdt_entry =
				u64::from_le_bytes(aligned_mem.mem[gdt_addr..gdt_addr + 8].try_into().unwrap());
			assert_eq!(*res, gdt_entry);
		}
	}
}
