#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
#[cfg_attr(target_arch = "aarch64", path = "aarch64.rs")]
mod arch;
pub use arch::*;
use raw_cpuid::CpuId;

pub fn init() {
    let cpuid = CpuId::new();

    if let Some(vendor_info) = cpuid.get_vendor_info() {
        log!("CPU {}\n", vendor_info);
    }

    if let Some(processor_brand_string) = cpuid.get_processor_brand_string() {
        log!("CPU {}\n", processor_brand_string.as_str().trim());
    }

    if let Some(processor_frequency_info) = cpuid.get_processor_frequency_info() {
        let processor_base_frequency = processor_frequency_info.processor_base_frequency();
        log!("CPU {} MHz\n", processor_base_frequency);
    }
}
