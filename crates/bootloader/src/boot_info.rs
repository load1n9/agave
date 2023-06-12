use crate::{
    arch::memory::Mapper,
    context::RuntimeContext,
    memory::{FrameAllocator, Page, PageRange, PteFlags},
};
use core::{alloc::Layout, mem::MaybeUninit, slice};
use uefi_bootloader_api::{BootInformation, ElfSection, FrameBuffer, MemoryRegion, Module};

impl RuntimeContext {
    pub(crate) fn create_boot_info(
        mut self,
        frame_buffer: Option<FrameBuffer>,
        rsdp_address: Option<usize>,
        modules: &'static [Module],
        elf_sections: &'static [ElfSection],
    ) -> &'static BootInformation {
        let boot_info_layout = Layout::new::<BootInformation>();

        let memory_regions_count = self.frame_allocator.len();
        let memory_regions_layout = Layout::array::<MemoryRegion>(memory_regions_count)
            .expect("failed to create memory regions layout");
        let (combined, memory_regions_offset) = boot_info_layout
            .extend(memory_regions_layout)
            .expect("failed to extend boot info layout with memory regions");

        let modules_layout =
            Layout::array::<Module>(modules.len()).expect("failed to create modules layout");
        let (combined, modules_offset) = combined
            .extend(modules_layout)
            .expect("failed to extend boot info layout with modules");

        let elf_sections_layout = Layout::array::<ElfSection>(elf_sections.len())
            .expect("failed to create elf sections layout");
        let (combined, elf_sections_offset) = combined
            .extend(elf_sections_layout)
            .expect("failed to extend boot info layout with elf sections");

        let boot_info_address = self.page_allocator.get_free_address(combined.size());

        let pages = PageRange::new(
            Page::containing_address(boot_info_address),
            Page::containing_address(boot_info_address + combined.size() - 1),
        );

        let mut bootloader_page_tables = Mapper::current(&mut self.frame_allocator);
        let flags = PteFlags::new().present(true).writable(true);

        for page in pages {
            let frame = self
                .frame_allocator
                .allocate_frame()
                .expect("failed to allocate boot info frame");
            self.mapper
                .map(page, frame, flags, &mut self.frame_allocator);
            bootloader_page_tables.map(page, frame, flags, &mut self.frame_allocator);
        }

        let memory_map_regions_address = boot_info_address + memory_regions_offset;
        let modules_address = boot_info_address + modules_offset;
        let elf_sections_address = boot_info_address + elf_sections_offset;

        let uninit_boot_info: &'static mut MaybeUninit<BootInformation> =
            // SAFETY: We allocated it.
            unsafe { &mut *(boot_info_address.value() as *mut _) };
        // SAFETY: We allocated it.
        let uninit_memory_regions: &'static mut [MaybeUninit<MemoryRegion>] = unsafe {
            slice::from_raw_parts_mut(
                memory_map_regions_address.value() as *mut _,
                memory_regions_count,
            )
        };
        let uninit_modules: &'static mut [MaybeUninit<Module>] =
            // SAFETY: We allocated it.
            unsafe { slice::from_raw_parts_mut(modules_address.value() as *mut _, modules.len()) };
        // SAFETY: We allocated it.
        let uninit_elf_sections: &'static mut [MaybeUninit<ElfSection>] = unsafe {
            slice::from_raw_parts_mut(elf_sections_address.value() as *mut _, elf_sections.len())
        };

        let memory_regions = self
            .frame_allocator
            .construct_memory_map(uninit_memory_regions)
            .into();
        let modules = MaybeUninit::write_slice(uninit_modules, modules).into();
        let elf_sections = MaybeUninit::write_slice(uninit_elf_sections, elf_sections).into();

        uninit_boot_info.write({
            BootInformation {
                size: combined.size(),
                frame_buffer,
                rsdp_address,
                memory_regions,
                modules,
                elf_sections,
            }
        })
    }
}
