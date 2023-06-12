// ported from https://github.com/theseus-os/Theseus/blob/theseus_main/kernel/boot_info/src/lib.rs
// #[cfg(feature = "multiboot2")]
// pub mod multiboot2;
// #[cfg(feature = "uefi")]
pub mod uefi;

use crate::api::memory::structs::{PhysicalAddress, VirtualAddress};
use core::iter::Iterator;

pub trait MemoryRegion {
    fn start(&self) -> PhysicalAddress;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn is_usable(&self) -> bool;
}

pub trait ElfSection {
    fn name(&self) -> &str;

    fn start(&self) -> VirtualAddress;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn flags(&self) -> ElfSectionFlags;
}

bitflags::bitflags! {
    pub struct ElfSectionFlags: u64 {
        const WRITABLE = 0x1;

        const ALLOCATED = 0x2;

        const EXECUTABLE = 0x4;
    }
}

pub trait Module {
    fn name(&self) -> Result<&str, &'static str>;

    fn start(&self) -> PhysicalAddress;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug)]
pub struct ReservedMemoryRegion {
    pub start: PhysicalAddress,
    pub len: usize,
}

#[derive(Debug)]
pub struct FramebufferInfo {
    pub virt_addr: Option<VirtualAddress>,
    pub phys_addr: PhysicalAddress,
    pub total_size_in_bytes: u64,
    pub width: u32,
    pub height: u32,
    pub bits_per_pixel: u8,
    pub stride: u32,
    pub format: FramebufferFormat,
}
impl FramebufferInfo {
    pub fn is_mapped(&self) -> bool {
        self.virt_addr.is_some()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum FramebufferFormat {
    RgbPixel,
    BgrPixel,
    Grayscale,
    TextCharacter,
    CustomPixel {
        red_bit_position: u8,
        red_size_in_bits: u8,
        green_bit_position: u8,
        green_size_in_bits: u8,
        blue_bit_position: u8,
        blue_size_in_bits: u8,
    },
}

pub trait BootInformation: 'static {
    type MemoryRegion<'a>: MemoryRegion;
    type MemoryRegions<'a>: Iterator<Item = Self::MemoryRegion<'a>>;

    type ElfSection<'a>: ElfSection;
    type ElfSections<'a>: Iterator<Item = Self::ElfSection<'a>>;

    type Module<'a>: Module;
    type Modules<'a>: Iterator<Item = Self::Module<'a>>;

    type AdditionalReservedMemoryRegions: Iterator<Item = ReservedMemoryRegion>;

    fn start(&self) -> Option<VirtualAddress>;
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn memory_regions(&self) -> Result<Self::MemoryRegions<'_>, &'static str>;
    fn elf_sections(&self) -> Result<Self::ElfSections<'_>, &'static str>;
    fn modules(&self) -> Self::Modules<'_>;

    fn additional_reserved_memory_regions(
        &self,
    ) -> Result<Self::AdditionalReservedMemoryRegions, &'static str>;

    fn kernel_end(&self) -> Result<VirtualAddress, &'static str>;

    fn rsdp(&self) -> Option<PhysicalAddress>;

    fn stack_size(&self) -> Result<usize, &'static str>;

    fn framebuffer_info(&self) -> Option<FramebufferInfo>;
}
