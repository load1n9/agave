use crate::{memory::VirtualAddress, BootContext};
use core::mem::MaybeUninit;
use goblin::elf64::{
    header::Header,
    program_header::{ProgramHeader, SIZEOF_PHDR},
    section_header::{SectionHeader, SIZEOF_SHDR},
};
use log::info;
use plain::Plain;
use uefi::{
    prelude::cstr16,
    proto::media::file::{File, FileAttribute, FileMode, FileType, RegularFile},
    table::boot::MemoryType,
    CStr16,
};
use uefi_bootloader_api::ElfSection;

const KERNEL_NAME: &CStr16 = cstr16!("kernel.elf");

impl BootContext {
    pub(crate) fn load_kernel(&mut self) -> (VirtualAddress, &'static mut [ElfSection]) {
        let mut root = self
            .open_file_system_root()
            .expect("failed to open file system root");

        let file = match root
            .open(KERNEL_NAME, FileMode::Read, FileAttribute::empty())
            .expect("failed to open kernel file")
            .into_type()
            .expect("kernel file was closed or deleted")
        {
            FileType::Regular(file) => file,
            FileType::Dir(_) => panic!(),
        };

        Loader {
            file,
            context: self,
        }
        .load()
    }
}

struct Loader<'a> {
    file: RegularFile,
    context: &'a mut BootContext,
}

impl Loader<'_> {
    fn load(mut self) -> (VirtualAddress, &'static mut [ElfSection]) {
        let mut buffer = [0; core::mem::size_of::<Header>()];
        self.file
            .read(&mut buffer)
            .expect("failed to read kernel header");

        let kernel_header = Header::from_bytes(&buffer);

        let program_header_offset = kernel_header.e_phoff;
        let program_header_count = kernel_header.e_phnum;

        let mut buffer = [0; SIZEOF_PHDR];

        for i in 0..program_header_count.into() {
            // Loading segments modifies the file position.
            self.file
                .set_position(program_header_offset + (i * SIZEOF_PHDR as u64))
                .expect("failed to set kernel file position to program header");
            self.file
                .read(&mut buffer)
                .expect("failed to read kernel program header");

            let program_header = ProgramHeader::from_bytes(&buffer)
                .expect("failed to create program header from bytes");

            // .got section
            if program_header.p_memsz == 0 {
                continue;
            }

            if program_header.p_type == 1 {
                self.handle_load_segment(program_header);
            }
        }

        (
            VirtualAddress::new_canonical(kernel_header.e_entry as usize),
            self.elf_sections(kernel_header),
        )
    }

    fn elf_sections(&mut self, header: &Header) -> &'static mut [ElfSection] {
        let program_header_count = header.e_shnum;

        // This slice is copied into another slice in the bootloader, so this slice can
        // be overwritten by the kernel.
        let sections = self
            .context
            .allocate_slice(program_header_count as usize, MemoryType::LOADER_DATA);
        let mut buffer = [0; SIZEOF_SHDR];

        let shstrtab_header = header.e_shoff + (u64::from(header.e_shstrndx) * SIZEOF_SHDR as u64);
        self.file
            .set_position(shstrtab_header)
            .expect("failed to set kernel file position to shstrtab header");
        self.file
            .read(&mut buffer)
            .expect("failed to read kernel shstrtab header");
        let shstrtab_section_header =
            SectionHeader::from_bytes(&buffer).expect("failed to create section header from bytes");
        let shstrtab_base = shstrtab_section_header.sh_offset;

        for (i, uninit_section) in sections.iter_mut().enumerate() {
            self.file
                .set_position(header.e_shoff + (i * SIZEOF_SHDR) as u64)
                .expect("failed to set kernel file position to section header");
            self.file
                .read(&mut buffer)
                .expect("failed to read kernel section header");
            let section_header = SectionHeader::from_bytes(&buffer)
                .expect("failed to create section header from bytes");

            let mut name = [0; 64];
            let name_position = shstrtab_base + u64::from(section_header.sh_name);
            self.file
                .set_position(name_position)
                .expect("failed to set kernel file position to shstrab name position");
            self.file
                .read(&mut name)
                .expect("failed to read kernel section name");

            uninit_section.write(ElfSection {
                name,
                start: section_header.sh_addr as usize,
                size: section_header.sh_size as usize,
                flags: section_header.sh_flags,
            });
        }

        // SAFETY: We initialised the sections.
        unsafe { MaybeUninit::slice_assume_init_mut(sections) }
    }

    fn handle_load_segment(&mut self, segment: &ProgramHeader) {
        info!("loading segment: {segment:?}");
        let slice = self.context.map_segment(segment);
        info!("at paddr: {:x?}", slice.as_ptr());

        self.file
            .set_position(segment.p_offset)
            .expect("failed to set kernel file position to segment offset");
        self.file
            .read(&mut slice[..segment.p_filesz as usize])
            .expect("failed to read kernel segment");

        // The BSS section was already zeroed by `map_segment`.
    }
}
