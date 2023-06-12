// ported from https://github.com/theseus-os/uefi-bootloader

#![allow(dead_code)]
#![feature(step_trait, abi_efiapi, maybe_uninit_slice, maybe_uninit_write_slice)]
#![no_std]
#![no_main]

mod arch;
mod boot_info;
mod context;
mod kernel;
mod logger;
mod mappings;
mod memory;
mod modules;
mod util;

use crate::{
    arch::jump_to_kernel,
    memory::{Frame, VirtualAddress},
};
use agave_uefi_bootloader_api::{BootInformation, FrameBuffer, FrameBufferInfo, PixelFormat};
use core::{fmt::Write, ptr::NonNull};
use log::{error, info};
use uefi::{
    prelude::entry,
    proto::console::gop::{self, GraphicsOutput},
    table::{
        cfg::{ACPI2_GUID, ACPI_GUID},
        Boot, SystemTable,
    },
    Handle, Status,
};

pub(crate) use context::{BootContext, RuntimeContext};

static mut SYSTEM_TABLE: Option<NonNull<SystemTable<Boot>>> = None;

#[entry]
fn main(handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    let system_table_pointer = NonNull::from(&mut system_table);
    // SAFETY: We are the sole thread.
    unsafe { SYSTEM_TABLE = Some(system_table_pointer) };

    system_table
        .stdout()
        .clear()
        .expect("failed to clear stdout");

    let mut frame_buffer = get_frame_buffer(&system_table);
    if let Some(frame_buffer) = frame_buffer {
        init_logger(&frame_buffer);
        info!("using framebuffer at {:#x}", frame_buffer.physical);
    }

    // SAFETY: We are the sole thread.
    unsafe { SYSTEM_TABLE = None };

    let rsdp_address = get_rsdp_address(&system_table);

    let mut context = BootContext::new(handle, system_table);
    let (entry_point, elf_sections) = context.load_kernel();
    info!("loaded kernel");
    // This may take a sec.
    info!("loading modules...");
    let modules = context.load_modules();
    info!("loaded modules");

    let mut context = context.exit_boot_services();

    let stack_top = context.set_up_mappings(frame_buffer.as_mut());
    info!("created memory mappings");

    let page_table_frame = context.page_table();
    info!(
        "page table located at: {:#x}",
        page_table_frame.start_address()
    );

    let boot_info = context.create_boot_info(frame_buffer, rsdp_address, modules, elf_sections);
    info!("created boot info: {boot_info:x?}");

    info!("about to jump to kernel: {:x?}", entry_point.value());
    // SAFETY: Everything is correctly mapped.
    unsafe {
        jump_to_kernel(KernelContext {
            page_table_frame,
            stack_top,
            entry_point,
            boot_info,
        })
    }
}

// The context necessary to switch to the kernel.
#[derive(Debug)]
struct KernelContext {
    page_table_frame: Frame,
    stack_top: VirtualAddress,
    entry_point: VirtualAddress,
    boot_info: &'static BootInformation,
}

fn get_frame_buffer(system_table: &SystemTable<Boot>) -> Option<FrameBuffer> {
    let handle = system_table
        .boot_services()
        .get_handle_for_protocol::<GraphicsOutput>()
        .ok()?;
    let mut gop = system_table
        .boot_services()
        .open_protocol_exclusive::<GraphicsOutput>(handle)
        .ok()?;

    let mode_info = gop.current_mode_info();
    let mut frame_buffer = gop.frame_buffer();
    let info = FrameBufferInfo {
        size: frame_buffer.size(),
        width: mode_info.resolution().0,
        height: mode_info.resolution().1,
        pixel_format: match mode_info.pixel_format() {
            gop::PixelFormat::Rgb => PixelFormat::Rgb,
            gop::PixelFormat::Bgr => PixelFormat::Bgr,
            gop::PixelFormat::Bitmask | gop::PixelFormat::BltOnly => {
                panic!("Bitmask and BltOnly framebuffers are not supported")
            }
        },
        bytes_per_pixel: 4,
        stride: mode_info.stride(),
    };

    Some(FrameBuffer {
        physical: frame_buffer.as_mut_ptr() as usize,
        virt: 0,
        info,
    })
}

fn init_logger(frame_buffer: &FrameBuffer) {
    // SAFETY: The hardware initialised the frame buffer.
    let slice = unsafe {
        core::slice::from_raw_parts_mut(frame_buffer.physical as *mut _, frame_buffer.info.size)
    };
    let logger =
        logger::LOGGER.call_once(move || logger::LockedLogger::new(slice, frame_buffer.info));
    log::set_logger(logger).expect("logger already set");
    log::set_max_level(log::LevelFilter::Trace);
}

fn get_rsdp_address(system_table: &SystemTable<Boot>) -> Option<usize> {
    let mut config_entries = system_table.config_table().iter();
    // look for an ACPI2 RSDP first
    let acpi2_rsdp = config_entries.find(|entry| matches!(entry.guid, ACPI2_GUID));
    // if no ACPI2 RSDP is found, look for a ACPI1 RSDP
    let rsdp = acpi2_rsdp.or_else(|| config_entries.find(|entry| matches!(entry.guid, ACPI_GUID)));
    rsdp.map(|entry| entry.address as usize)
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    // SAFETY: We are the sole thread.
    if let Some(mut system_table_pointer) = unsafe { SYSTEM_TABLE } {
        // SAFETY: We are the sole thread.
        let system_table = unsafe { system_table_pointer.as_mut() };
        let _ = writeln!(system_table.stdout(), "{info}");
    }

    if let Some(logger) = logger::LOGGER.get() {
        // SAFETY: We are the sole thread.
        unsafe { logger.force_unlock() };
    }
    error!("{info}");

    arch::halt();
}
