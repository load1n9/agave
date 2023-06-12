use crate::{memory::PAGE_SIZE, util::calculate_pages, BootContext};
use core::mem::MaybeUninit;
use uefi::{
    prelude::cstr16,
    proto::media::file::{File, FileAttribute, FileMode},
    table::boot::MemoryType,
};
use uefi_bootloader_api::Module;

const MODULES_MEMORY: MemoryType = MemoryType::custom(0x8000_0000);

impl BootContext {
    pub(crate) fn load_modules(&self) -> &'static mut [Module] {
        let mut root = self
            .open_file_system_root()
            .expect("failed to open file system root");

        let mut dir = match root.open(cstr16!("modules"), FileMode::Read, FileAttribute::empty()) {
            Ok(dir) => dir
                .into_directory()
                .expect("modules directory was closed or deleted"),
            Err(_) => return &mut [],
        };

        let mut num_modules = 0;
        let mut num_pages = 0;
        let mut buf = [0; 500];

        while let Some(info) = dir
            .read_entry(&mut buf)
            .expect("failed to read modules directory entry")
        {
            if !info.attribute().contains(FileAttribute::DIRECTORY) {
                num_modules += 1;
                // Theseus modules must not share pages i.e. the next module starts on a new
                // page.
                num_pages += calculate_pages(info.file_size() as usize);
            }
        }

        // This slice is copied into another slice in the bootloader, so this slice can
        // be overwritten by the kernel.
        let modules = self.allocate_slice(num_modules, MemoryType::LOADER_DATA);
        let raw_bytes = self.allocate_byte_slice(num_pages * PAGE_SIZE, MODULES_MEMORY);

        dir.reset_entry_readout()
            .expect("failed to reset modules directory entry readout");

        let mut idx = 0;
        let mut num_pages = 0;

        while let Some(info) = dir
            .read_entry(&mut buf)
            .expect("failed to read modules directory entry")
        {
            if !info.attribute().contains(FileAttribute::DIRECTORY) {
                let name = info.file_name();

                let len = info.file_size() as usize;
                let mut file = dir
                    .open(info.file_name(), FileMode::Read, FileAttribute::empty())
                    .expect("failed to open module")
                    .into_regular_file()
                    .expect("module file was closed or deleted");

                file.read(&mut raw_bytes[(num_pages * 4096)..])
                    .expect("failed to read module");

                let mut name_buf = [0; 64];
                let mut name_idx = 0;
                for c16 in name.iter() {
                    let c = char::from(*c16);
                    let s = c.encode_utf8(&mut name_buf[name_idx..(name_idx + 4)]);
                    name_idx += s.len();
                }

                modules[idx].write(Module {
                    name: name_buf,
                    offset: num_pages * 4096,
                    len,
                });

                idx += 1;
                num_pages += calculate_pages(len);
            }
        }

        assert_eq!(idx, modules.len());
        unsafe { MaybeUninit::slice_assume_init_mut(modules) }
    }
}
