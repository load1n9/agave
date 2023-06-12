use crate::{
    memory::{Frame, FrameAllocator, Page, PhysicalAddress, VirtualAddress},
    RuntimeContext,
};
use bit_field::BitField;
use goblin::elf64::program_header::ProgramHeader;
use x86_64::{
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{self, OffsetPageTable, PageTable, PageTableIndex},
};

pub(crate) fn is_canonical_virtual_address(virt_addr: usize) -> bool {
    matches!(virt_addr.get_bits(47..64), 0 | 0b1_1111_1111_1111_1111)
}

#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub(crate) const fn canonicalize_virtual_address(virt_addr: usize) -> usize {
    // match virt_addr.get_bit(47) {
    //     false => virt_addr.set_bits(48..64, 0),
    //     true =>  virt_addr.set_bits(48..64, 0xffff),
    // };

    // The below code is semantically equivalent to the above, but it works in const
    // functions.
    ((virt_addr << 16) as isize >> 16) as usize
}

pub(crate) fn is_canonical_physical_address(phys_addr: usize) -> bool {
    phys_addr.get_bits(52..64) == 0
}

pub(crate) const fn canonicalize_physical_address(phys_addr: usize) -> usize {
    phys_addr & 0x000F_FFFF_FFFF_FFFF
}

pub(crate) fn set_up_arch_specific_mappings(context: &mut RuntimeContext) {
    let p4_frame = paging::PhysFrame::from_start_address(x86_64::PhysAddr::new(
        context.mapper.inner.level_4_table() as *const _ as u64,
    ))
    .expect("invalid p4 frame");

    #[allow(clippy::inconsistent_digit_grouping)]
    let p4_index = x86_64::VirtAddr::new(0o177777_776_000_000_000_0000).p4_index();
    let entry = &mut context.mapper.inner.level_4_table()[p4_index];
    entry.set_frame(
        p4_frame,
        paging::PageTableFlags::PRESENT | paging::PageTableFlags::WRITABLE,
    );
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PteFlags(u64);

impl PteFlags {
    pub(crate) fn new() -> Self {
        Self(0)
    }

    pub(crate) fn present(self, enable: bool) -> Self {
        const BITS: u64 = paging::PageTableFlags::PRESENT.bits();

        if enable {
            Self(self.0 | BITS)
        } else {
            Self(self.0 & !(BITS))
        }
    }

    pub(crate) fn writable(self, enable: bool) -> Self {
        const BITS: u64 = paging::PageTableFlags::WRITABLE.bits();

        if enable {
            Self(self.0 | BITS)
        } else {
            Self(self.0 & !(BITS))
        }
    }

    pub(crate) fn no_execute(self, enable: bool) -> Self {
        const BITS: u64 = paging::PageTableFlags::NO_EXECUTE.bits();

        if enable {
            Self(self.0 | BITS)
        } else {
            Self(self.0 & !(BITS))
        }
    }
}

impl From<PteFlags> for paging::PageTableFlags {
    fn from(flags: PteFlags) -> Self {
        paging::PageTableFlags::from_bits_truncate(flags.0)
    }
}

impl From<x86_64::VirtAddr> for VirtualAddress {
    fn from(value: x86_64::VirtAddr) -> Self {
        Self::new_canonical(value.as_u64() as usize)
    }
}

impl From<x86_64::PhysAddr> for PhysicalAddress {
    fn from(value: x86_64::PhysAddr) -> Self {
        Self::new_canonical(value.as_u64() as usize)
    }
}

impl From<Page> for paging::Page {
    fn from(page: Page) -> Self {
        Self::from_start_address(x86_64::VirtAddr::new(page.start_address().value() as u64))
            .expect("failed to convert page to x86_64 page")
    }
}

impl From<Frame> for paging::PhysFrame {
    fn from(frame: Frame) -> Self {
        Self::from_start_address(x86_64::PhysAddr::new(frame.start_address().value() as u64))
            .expect("failed to convert frame to x86_64 frame")
    }
}

// Implement other functions for the `Page` type that aren't relevant for
// `Frame.
impl Page {
    /// Returns the 9-bit part of this `Page`'s [`VirtualAddress`] that is the
    /// index into the P4 page table entries list.
    const fn p4_index(self) -> usize {
        (self.number >> 27) & 0x1FF
    }
}

pub(crate) struct PageAllocator {
    level_4_entries: [bool; 512],
}

impl PageAllocator {
    pub(crate) fn new() -> Self {
        let mut page_allocator = Self {
            level_4_entries: [false; 512],
        };
        page_allocator.level_4_entries[0] = true;

        page_allocator
    }

    fn get_free_entries(&mut self, num: u64) -> PageTableIndex {
        // Create an iterator over all available p4 indices with `num` contiguous free
        // entries.
        let mut free_entries = self
            .level_4_entries
            .windows(num as usize)
            .enumerate()
            .filter(|(_, entries)| entries.iter().all(|used| !used))
            .map(|(idx, _)| idx);

        let idx = free_entries
            .next()
            .expect("no usable level 4 entries found");

        // Mark the entries as used.
        for i in 0..num as usize {
            self.level_4_entries[idx + i] = true;
        }

        PageTableIndex::new(
            idx.try_into()
                .expect("page table index larger than u16::MAX"),
        )
    }

    pub(crate) fn get_free_address(&mut self, len: usize) -> VirtualAddress {
        const LEVEL_4_SIZE: usize = 4096 * 512 * 512 * 512;
        let num_level_4_entries = (len + (LEVEL_4_SIZE - 1)) / LEVEL_4_SIZE;

        // This is technically a 512 GiB page.
        paging::Page::from_page_table_indices_1gib(
            self.get_free_entries(num_level_4_entries as u64),
            PageTableIndex::new(0),
        )
        .start_address()
        .into()
    }

    pub(crate) fn mark_segment_as_used(&mut self, segment: &ProgramHeader) {
        let start = VirtualAddress::new_canonical(segment.p_vaddr as usize);
        let end_inclusive = (start + segment.p_memsz as usize) - 1;

        let start_page = Page::containing_address(start);
        let end_page_inclusive = Page::containing_address(end_inclusive);

        for p4_index in start_page.p4_index()..=end_page_inclusive.p4_index() {
            self.level_4_entries[p4_index] = true;
        }
    }
}

struct FrameAllocatorWrapper<'a, T>
where
    T: FrameAllocator,
{
    inner: &'a mut T,
}

// SAFETY: This returns a unique unused frame.
unsafe impl<T> paging::FrameAllocator<paging::page::Size4KiB> for FrameAllocatorWrapper<'_, T>
where
    T: FrameAllocator,
{
    fn allocate_frame(&mut self) -> Option<paging::PhysFrame<paging::page::Size4KiB>> {
        FrameAllocator::allocate_frame(self.inner).map(Frame::into)
    }
}

pub(crate) struct Mapper {
    inner: OffsetPageTable<'static>,
}

impl Mapper {
    pub(crate) fn new<T>(frame_allocator: &mut T) -> Self
    where
        T: FrameAllocator,
    {
        let frame = frame_allocator
            .allocate_frame()
            .expect("failed to allocate frame for page table");
        // Physical memory is identity-mapped.
        let pointer = frame.start_address().value() as *mut PageTable;
        // SAFETY: It is a valid, page-aligned pointer.
        unsafe { pointer.write(PageTable::new()) };
        // SAFETY: We initialised the value.
        let level_4_table = unsafe { &mut *pointer };
        Self {
            // SAFETY: The physical offset is zero.
            inner: unsafe { OffsetPageTable::new(level_4_table, x86_64::VirtAddr::zero()) },
        }
    }

    pub(crate) fn current<T>(frame_allocator: &mut T) -> Self
    where
        T: FrameAllocator,
    {
        // We copy the old table as some loaders mark the top-level page table as
        // read-only.
        let old_table = {
            let frame = Cr3::read_raw().0;
            let pointer = frame.start_address().as_u64() as *mut PageTable;
            // SAFETY: The pointer is valid as physical memory is identity-mapped.
            unsafe { &*pointer }
        };

        let new_frame = frame_allocator
            .allocate_frame()
            .expect("failed to allocate frame for page table");
        let new_table = {
            let pointer = new_frame.start_address().value() as *mut PageTable;
            // SAFETY: The pointer is valid as physical memory is identity-mapped.
            unsafe {
                pointer.write(PageTable::new());
                &mut *pointer
            }
        };
        // Only the first P3 table is relevant as we have less than 512GiB of memory.
        new_table[0] = old_table[0].clone();

        // SAFETY: The table is the same (at least for the first 512GiB).
        unsafe { Cr3::write(new_frame.into(), Cr3Flags::empty()) };
        Self {
            // SAFETY: The physical offset is zero.
            inner: unsafe { OffsetPageTable::new(new_table, x86_64::VirtAddr::zero()) },
        }
    }

    // TODO: This should take a shared reference to self.
    pub(crate) fn frame(&mut self) -> Frame {
        Frame::containing_address(PhysicalAddress::new_canonical(self.inner.level_4_table()
            as *const _
            as usize))
    }

    pub(crate) fn map<T>(
        &mut self,
        page: Page,
        frame: Frame,
        flags: PteFlags,
        frame_allocator: &mut T,
    ) where
        T: FrameAllocator,
    {
        // SAFETY: 🤷
        unsafe {
            paging::Mapper::<paging::Size4KiB>::map_to(
                &mut self.inner,
                page.into(),
                frame.into(),
                flags.into(),
                &mut FrameAllocatorWrapper {
                    inner: frame_allocator,
                },
            )
        }
        .expect("failed to map page to frame")
        // TODO: Do we need to flush everytime?
        .flush();
    }
}
