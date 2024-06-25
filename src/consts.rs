pub const PAGE_SIZE: usize = 0x1000;
pub const GDT_KERNEL_CODE: u16 = 1;
pub const GDT_KERNEL_DATA: u16 = 2;
pub const APIC_DEFAULT_BASE: u64 = 0xfee00000;

pub const BOOT_GDT_NULL: usize = 0;
pub const BOOT_GDT_CODE: usize = 1;
pub const BOOT_GDT_DATA: usize = 2;
pub const BOOT_GDT_MAX: usize = 3;

// guest_address + OFFSET
pub const GDT_OFFSET: u64 = 0x1000;
pub const FDT_OFFSET: u64 = 0x5000;
pub const BOOT_INFO_OFFSET: u64 = 0x9000;
pub const PML4_OFFSET: u64 = 0x10000;
pub const PGT_OFFSET: u64 = 0x10000;
pub const PDPTE_OFFSET: u64 = 0x11000;
pub const PDE_OFFSET: u64 = 0x12000;
pub const MIN_PHYSMEM_SIZE: usize = 0x13000;

pub const EFER_SCE: u64 = 1; /* System Call Extensions */
pub const EFER_LME: u64 = 1 << 8; /* Long mode enable */
pub const EFER_LMA: u64 = 1 << 10; /* Long mode active (read-only) */
pub const EFER_NXE: u64 = 1 << 11; /* PTE No-Execute bit enable */
pub const IOAPIC_BASE: u64 = 0xfec00000;
pub const IOAPIC_SIZE: u64 = 0x1000;
pub const KERNEL_STACK_SIZE: u64 = 32_768;
pub const SHAREDQUEUE_START: usize = 0x80000;
pub const UHYVE_NET_MTU: usize = 1500;
pub const UHYVE_QUEUE_SIZE: usize = 8;
pub const UHYVE_IRQ_NET: u32 = 11;

pub const GUEST_PAGE_SIZE: u64 = 0x200000; /* 2 MB pages in guest */
