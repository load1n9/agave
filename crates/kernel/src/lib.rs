#![no_std]
#![cfg_attr(test, no_main)]
#![feature(abi_x86_interrupt)]
#![feature(doc_cfg)]
#![feature(error_in_core)]
#![feature(step_trait)]
#![feature(alloc_error_handler)]
#![feature(naked_functions)]
#![feature(never_type)]
#![feature(exact_size_is_empty)]
#![feature(custom_test_frameworks)]
#![feature(ptr_internals)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
extern crate lazy_static;

use bootloader_api::info::FrameBufferInfo;
use bootloader_x86_64_common::logger::LockedLogger;
use conquer_once::spin::OnceCell;

pub static LOGGER: OnceCell<LockedLogger> = OnceCell::uninit();

pub fn init_logger(buffer: &'static mut [u8], info: FrameBufferInfo) {
    let logger = LOGGER.get_or_init(move || LockedLogger::new(buffer, info, true, false));
    log::set_logger(logger).expect("Logger already set");
    log::set_max_level(log::LevelFilter::Trace);
    log::info!(
        "AGAVE v{}\n",
        option_env!("AGAVE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
    );
}

#[macro_use]
pub mod api;

#[macro_use]
pub mod sys;

pub mod usr;
use bootloader_api::BootInfo;

const KERNEL_SIZE: usize = 2 << 20; // 2 MB

pub fn init(boot_info: &'static mut BootInfo) {
    // sys::vga::init();
    let _memory_regions = &boot_info.memory_regions;
    let _physical_memory_offset = &boot_info.physical_memory_offset;
    // sys::mem::init(_memory_regions, _physical_memory_offset);
    let frame_buffer_optional = &mut boot_info.framebuffer;
    let frame_buffer_option = frame_buffer_optional.as_mut();
    let frame_buffer_struct = frame_buffer_option.unwrap();
    let frame_buffer_info = frame_buffer_struct.info().clone();
    let raw_frame_buffer = frame_buffer_struct.buffer_mut();
    init_logger(raw_frame_buffer, frame_buffer_info);
    sys::gdt::init();
    sys::idt::init();
    sys::pic::init(); // Enable interrupts
    sys::serial::init();
    sys::keyboard::init();
    sys::time::init();

    log!(
        "AGAVE v{}\n",
        option_env!("AGAVE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
    );
    sys::cpu::init();
    sys::pci::init(); // Require MEM
    sys::net::init(); // Require PCI
    sys::ata::init();
    sys::fs::init(); // Require ATA
    sys::clock::init(); // Require MEM
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    let csi_color = api::console::Style::color("LightRed");
    let csi_reset = api::console::Style::reset();
    printk!(
        "{}Error:{} Could not allocate {} bytes\n",
        csi_color,
        csi_reset,
        layout.size()
    );
    hlt_loop();
}

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        print!("test {} ... ", core::any::type_name::<T>());
        self();
        let csi_color = api::console::Style::color("LightGreen");
        let csi_reset = api::console::Style::reset();
        println!("{}ok{}", csi_color, csi_reset);
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    let n = tests.len();
    println!("\nrunning {} test{}", n, if n == 1 { "" } else { "s" });
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    #[cfg(feature = "x86_64")]
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        #[cfg(feature = "x86_64")]
        port.write(exit_code as u32);
    }
}

pub fn hlt_loop() -> ! {
    loop {
        #[cfg(feature = "x86_64")]
        x86_64::instructions::hlt();
    }
}

#[cfg(test)]
use bootloader_api::entry_point;

#[cfg(test)]
use core::panic::PanicInfo;

#[cfg(test)]
entry_point!(test_kernel_main);

#[cfg(test)]
fn test_kernel_main(boot_info: &'static mut BootInfo) -> ! {
    init(boot_info);
    test_main();
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let csi_color = api::console::Style::color("LightRed");
    let csi_reset = api::console::Style::reset();
    println!("{}failed{}\n", csi_color, csi_reset);
    println!("{}\n", info);
    exit_qemu(QemuExitCode::Failed);
    hlt_loop();
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
