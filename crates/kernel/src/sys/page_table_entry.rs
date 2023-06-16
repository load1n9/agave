// ported from https://github.com/theseus-os/Theseus/blob/theseus_main/kernel/page_table_entry/src/lib.rs

use super::frame_alloc::AllocatedFrame;
use super::pte_flags::{PteFlagsArch, PTE_FRAME_MASK};
use crate::api::memory::structs::{Frame, FrameRange, PhysicalAddress};
use core::ops::Deref;
use zerocopy::FromBytes;

#[derive(FromBytes)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    pub fn is_unused(&self) -> bool {
        self.0 == 0
    }

    pub fn zero(&mut self) {
        self.0 = 0;
    }

    pub fn set_unmapped(&mut self) -> UnmapResult {
        let frame = self.frame_value();
        let flags = self.flags();
        self.zero();

        let frame_range = FrameRange::new(frame, frame);
        if flags.is_exclusive() {
            UnmapResult::Exclusive(UnmappedFrames(frame_range))
        } else {
            UnmapResult::NonExclusive(frame_range)
        }
    }

    pub fn flags(&self) -> PteFlagsArch {
        PteFlagsArch::from_bits_truncate(self.0 & !PTE_FRAME_MASK)
    }

    pub fn pointed_frame(&self) -> Option<Frame> {
        if self.flags().is_valid() {
            Some(self.frame_value())
        } else {
            None
        }
    }

    fn frame_value(&self) -> Frame {
        let mut frame_paddr = self.0 as usize;
        frame_paddr &= PTE_FRAME_MASK as usize;
        Frame::containing_address(PhysicalAddress::new_canonical(frame_paddr))
    }

    pub fn set_entry(&mut self, frame: AllocatedFrame, flags: PteFlagsArch) {
        self.0 = (frame.start_address().value() as u64) | flags.bits();
    }

    pub fn set_flags(&mut self, new_flags: PteFlagsArch) {
        let only_flag_bits = new_flags.bits() & !PTE_FRAME_MASK;
        self.0 = (self.0 & PTE_FRAME_MASK) | only_flag_bits;
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

#[must_use]
pub enum UnmapResult {
    Exclusive(UnmappedFrames),
    NonExclusive(FrameRange),
}

pub struct UnmappedFrames(FrameRange);
impl Deref for UnmappedFrames {
    type Target = FrameRange;
    fn deref(&self) -> &FrameRange {
        &self.0
    }
}
