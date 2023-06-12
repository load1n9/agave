// ported from https://github.com/theseus-os/uefi-bootloader
#![feature(pointer_byte_offsets)]
#![no_std]

use core::{ops, slice, str};

#[derive(Debug)]
#[repr(C)]
pub struct BootInformation {
    pub size: usize,
    pub frame_buffer: Option<FrameBuffer>,
    pub rsdp_address: Option<usize>,
    pub memory_regions: MemoryRegions,
    pub modules: Modules,
    pub elf_sections: ElfSections,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FrameBuffer {
    /// The framebuffer's physical address.
    pub physical: usize,
    /// The framebuffer's virtual address.
    pub virt: usize,
    pub info: FrameBufferInfo,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FrameBufferInfo {
    pub size: usize,
    pub width: usize,
    pub height: usize,
    pub pixel_format: PixelFormat,
    pub bytes_per_pixel: usize,
    pub stride: usize,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub enum PixelFormat {
    Rgb,
    Bgr,
}

/// FFI-safe slice of [`MemoryRegion`] structs, semantically equivalent to
/// `&'static mut [MemoryRegion]`.
#[derive(Debug)]
#[repr(C)]
pub struct MemoryRegions {
    pub(crate) ptr: *mut MemoryRegion,
    pub(crate) len: usize,
}

impl ops::Deref for MemoryRegions {
    type Target = [MemoryRegion];

    fn deref(&self) -> &Self::Target {
        // SAFETY: Pointer and length were calculated from a valid slice.
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl ops::DerefMut for MemoryRegions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: Pointer and length were calculated from a valid slice.
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl From<&'static mut [MemoryRegion]> for MemoryRegions {
    fn from(regions: &'static mut [MemoryRegion]) -> Self {
        MemoryRegions {
            ptr: regions.as_mut_ptr(),
            len: regions.len(),
        }
    }
}

impl From<MemoryRegions> for &'static mut [MemoryRegion] {
    fn from(regions: MemoryRegions) -> &'static mut [MemoryRegion] {
        // SAFETY: Pointer and length were calculated from a valid slice.
        unsafe { slice::from_raw_parts_mut(regions.ptr, regions.len) }
    }
}

/// Represent a physical memory region.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
pub struct MemoryRegion {
    /// The physical start address of the region.
    pub start: usize,
    /// The physical end address (exclusive) of the region.
    pub len: usize,
    /// The memory type of the memory region.
    ///
    /// Only [`Usable`][MemoryRegionKind::Usable] regions can be freely used.
    pub kind: MemoryRegionKind,
}

impl MemoryRegion {
    /// Creates a new empty memory region (with length 0).
    #[must_use]
    pub const fn empty() -> Self {
        MemoryRegion {
            start: 0,
            len: 0,
            kind: MemoryRegionKind::Bootloader,
        }
    }
}

/// Represents the different types of memory.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[non_exhaustive]
#[repr(C)]
pub enum MemoryRegionKind {
    /// Unused conventional memory, can be used by the kernel.
    Usable,
    /// Memory mappings created by the bootloader, including the page table and
    /// boot info mappings.
    ///
    /// This memory should _not_ be used by the kernel.
    Bootloader,
    /// An unknown memory region reported by the UEFI firmware.
    ///
    /// Contains the UEFI memory type tag.
    UnknownUefi(u32),
}

/// FFI-safe slice of [`Module`] structs, semantically equivalent to `&'static
/// mut [Module]`.
#[derive(Debug)]
#[repr(C)]
pub struct Modules {
    pub(crate) ptr: *mut Module,
    pub(crate) len: usize,
}

impl ops::Deref for Modules {
    type Target = [Module];

    fn deref(&self) -> &Self::Target {
        // SAFETY: Pointer and length were calculated from a valid slice.
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl ops::DerefMut for Modules {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: Pointer and length were calculated from a valid slice.
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl From<&'static mut [Module]> for Modules {
    fn from(modules: &'static mut [Module]) -> Self {
        Self {
            ptr: modules.as_mut_ptr(),
            len: modules.len(),
        }
    }
}

impl From<Modules> for &'static mut [Module] {
    fn from(modules: Modules) -> Self {
        // SAFETY: Pointer and length were calculated from a valid slice.
        unsafe { slice::from_raw_parts_mut(modules.ptr, modules.len) }
    }
}

/// A file.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Module {
    /// The name of the module encoded as a null-terminated UTF-8 string.
    #[doc(hidden)]
    pub name: [u8; 64],
    /// The offset in bytes from the start of the modules.
    ///
    /// The offset is guaranteed to be page aligned.
    pub offset: usize,
    /// The length of the module in bytes.
    pub len: usize,
}

impl Module {
    /// The name of the module.
    #[must_use]
    pub fn name(&self) -> &str {
        let end = self
            .name
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(self.name.len());
        str::from_utf8(&self.name[..end]).expect("invalid bytes in module name")
    }
}

/// FFI-safe slice of [`ElfSection`] structs, semantically equivalent to
/// `&'static mut [ElfSection]`.
#[derive(Debug)]
#[repr(C)]
pub struct ElfSections {
    pub(crate) ptr: *mut ElfSection,
    pub(crate) len: usize,
}

impl ops::Deref for ElfSections {
    type Target = [ElfSection];

    fn deref(&self) -> &Self::Target {
        // SAFETY: Pointer and length were calculated from a valid slice.
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl ops::DerefMut for ElfSections {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: Pointer and length were calculated from a valid slice.
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl From<&'static mut [ElfSection]> for ElfSections {
    fn from(elf_sections: &'static mut [ElfSection]) -> Self {
        Self {
            ptr: elf_sections.as_mut_ptr(),
            len: elf_sections.len(),
        }
    }
}

impl From<ElfSections> for &'static mut [ElfSection] {
    fn from(elf_sections: ElfSections) -> Self {
        // SAFETY: Pointer and length were calculated from a valid slice.
        unsafe { slice::from_raw_parts_mut(elf_sections.ptr, elf_sections.len) }
    }
}

/// An ELF section.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ElfSection {
    /// The name of the section encoded as a null-terminated UTF-8 string.
    #[doc(hidden)]
    pub name: [u8; 64],
    /// The starting virtual address of the section.
    pub start: usize,
    /// The size of the section in bytes.
    pub size: usize,
    /// The section flags.
    pub flags: u64,
}

impl ElfSection {
    /// The name of the section.
    #[must_use]
    pub fn name(&self) -> &str {
        let end = self
            .name
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(self.name.len());
        str::from_utf8(&self.name[..end]).expect("invalid bytes in section name")
    }
}