#[cfg(feature = "x86_64")]
const fn canonicalize(addr: usize) -> usize {
    addr | 0xFFFF_0000_0000_0000
}

pub const BYTES_PER_ADDR: usize = core::mem::size_of::<usize>();

pub const PAGE_SHIFT: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;

pub const P1_INDEX_SHIFT: usize = 0;
pub const P2_INDEX_SHIFT: usize = P1_INDEX_SHIFT + 9;
pub const P3_INDEX_SHIFT: usize = P2_INDEX_SHIFT + 9;
pub const P4_INDEX_SHIFT: usize = P3_INDEX_SHIFT + 9;

pub const ADDRESSABILITY_PER_P4_ENTRY: usize = 1 << (PAGE_SHIFT + P4_INDEX_SHIFT);

pub const MAX_VIRTUAL_ADDRESS: usize = canonicalize(usize::MAX);

pub const TEMPORARY_PAGE_VIRT_ADDR: usize = MAX_VIRTUAL_ADDRESS;

pub const ENTRIES_PER_PAGE_TABLE: usize = PAGE_SIZE / BYTES_PER_ADDR;
pub const KERNEL_TEXT_P4_INDEX: usize = ENTRIES_PER_PAGE_TABLE - 1;

pub const RECURSIVE_P4_INDEX: usize = ENTRIES_PER_PAGE_TABLE - 2;
pub const KERNEL_HEAP_P4_INDEX: usize = ENTRIES_PER_PAGE_TABLE - 3;

pub const UPCOMING_PAGE_TABLE_RECURSIVE_P4_INDEX: usize = ENTRIES_PER_PAGE_TABLE - 4;

pub const MAX_PAGE_NUMBER: usize = MAX_VIRTUAL_ADDRESS / PAGE_SIZE;

#[cfg(not(debug_assertions))]
pub const KERNEL_STACK_SIZE_IN_PAGES: usize = 16;
#[cfg(debug_assertions)]
pub const KERNEL_STACK_SIZE_IN_PAGES: usize = 32;

const TWO_GIGABYTES: usize = 0x8000_0000;

pub const KERNEL_OFFSET: usize = canonicalize(MAX_VIRTUAL_ADDRESS - (TWO_GIGABYTES - 1));

pub const KERNEL_TEXT_START: usize =
    canonicalize(KERNEL_TEXT_P4_INDEX << (P4_INDEX_SHIFT + PAGE_SHIFT));

pub const KERNEL_TEXT_MAX_SIZE: usize = ADDRESSABILITY_PER_P4_ENTRY - TWO_GIGABYTES;

pub const KERNEL_HEAP_START: usize =
    canonicalize(KERNEL_HEAP_P4_INDEX << (P4_INDEX_SHIFT + PAGE_SHIFT));

#[cfg(not(debug_assertions))]
pub const KERNEL_HEAP_INITIAL_SIZE: usize = 64 * 1024 * 1024;

#[cfg(debug_assertions)]
pub const KERNEL_HEAP_INITIAL_SIZE: usize = 256 * 1024 * 1024;

pub const KERNEL_HEAP_MAX_SIZE: usize = ADDRESSABILITY_PER_P4_ENTRY;

pub const UPCOMING_PAGE_TABLE_RECURSIVE_MEMORY_START: usize =
    canonicalize(UPCOMING_PAGE_TABLE_RECURSIVE_P4_INDEX << (P4_INDEX_SHIFT + PAGE_SHIFT));
