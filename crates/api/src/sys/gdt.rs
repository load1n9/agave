use lazy_static::lazy_static;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use x86_64::instructions::{
    segmentation::{Segment, CS, DS, ES, SS},
    tables::load_tss,
};
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use x86_64::structures::{
    gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
    tss::TaskStateSegment,
};
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
pub const PAGE_FAULT_IST_INDEX: u16 = 1;
pub const GENERAL_PROTECTION_FAULT_IST_INDEX: u16 = 2;
const STACK_SIZE: usize = 4 * 1024; //4096 * 5; other option: const STACK_SIZE: usize = 1024 * 8;

lazy_static! {
    pub static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let data_selector = gdt.append(Descriptor::kernel_data_segment());
        let user_code_selector = gdt.append(Descriptor::user_code_segment());
        let user_data_selector = gdt.append(Descriptor::user_data_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));
        (
            gdt,
            Selectors {
                code_selector,
                tss_selector,
                data_selector,
                user_code_selector,
                user_data_selector,
            },
        )
    };
}
pub struct Selectors {
    code_selector: SegmentSelector,
    data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
}

lazy_static! {
    pub static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            #[allow(static_mut_refs)]
            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            let stack_end = stack_start + STACK_SIZE as u64;
            stack_end
        };
        tss
    };
}

pub fn init() {
    GDT.0.load();
    unsafe {
        // CS::set_reg(code_selector);
        DS::set_reg(GDT.1.data_selector);
        ES::set_reg(GDT.1.data_selector);
        SS::set_reg(GDT.1.data_selector);
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}
