#![no_std]
#![no_main]

use acpi::{AcpiTables, HpetInfo, InterruptModel};
use agave_api::sys::{
    allocator, drivers,
    framebuffer::FB,
    gdt, globals, interrupts, ioapic, local_apic,
    logger::init_logger,
    memory::{self, BootInfoFrameAllocator},
    pci,
    task::{self, executor::yield_once},
    virtio::{DeviceType, Virtio},
    wasm::WasmApp,
    with_mapper_framealloc, ACPI_HANDLER, FRAME_ALLOCATOR, MAPPER, VIRTUAL_MAPPING_OFFSET,
};
use alloc::{boxed::Box, slice, vec::Vec};
use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use bootloader_boot_config::LevelFilter;
use core::panic::PanicInfo;
use spin::Mutex;
use x86_64::{
    structures::paging::{Mapper, Page, PageTableFlags, PhysFrame, Size1GiB, Size2MiB},
    PhysAddr, VirtAddr,
};

extern crate agave_api;
extern crate alloc;

const CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config.kernel_stack_size = 128 * 1024;
    config
};

entry_point!(main, config = &CONFIG);

#[allow(unused_variables)]
fn main(boot_info: &'static mut BootInfo) -> ! {
    gdt::init();
    interrupts::init_idt();

    let framebuffer = boot_info.framebuffer.as_mut().unwrap();
    let fbinfo = framebuffer.info();

    let fbm = framebuffer.buffer_mut();
    let fbm2 = unsafe {
        let p = fbm.as_mut_ptr();
        slice::from_raw_parts_mut(p, fbinfo.byte_len)
    };

    init_logger(fbm, fbinfo.clone(), LevelFilter::Trace, true, true);

    let virtual_full_mapping_offset = VirtAddr::new(
        boot_info
            .physical_memory_offset
            .into_option()
            .expect("no physical_memory_offset"),
    );
    // log::info!("physical_memory_offset {:x}", virtual_full_mapping_offset);
    unsafe {
        VIRTUAL_MAPPING_OFFSET = virtual_full_mapping_offset;
    }
    let mapper = unsafe { memory::init(virtual_full_mapping_offset) };
    let frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    MAPPER.init_once(|| Mutex::new(mapper));
    FRAME_ALLOCATOR.init_once(|| Mutex::new(frame_allocator));
    {
        // log::info!("Complete Bootloader Map physical memory");
        type VirtualMappingPageSize = Size2MiB; // Size2MiB;Size1GiB Size4KiB

        let start_frame: PhysFrame<VirtualMappingPageSize> =
            PhysFrame::containing_address(PhysAddr::new(0));
        let _max_phys = PhysAddr::new(virtual_full_mapping_offset.as_u64() - 1u64);
        let max_phys = PhysAddr::new(Size1GiB::SIZE * 64 - 1);

        let end_frame: PhysFrame<VirtualMappingPageSize> = PhysFrame::containing_address(max_phys);

        use x86_64::structures::paging::PageSize;
        let mut news = 0;
        let mut olds = 0;
        for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
            let page: Page<VirtualMappingPageSize> = Page::containing_address(
                virtual_full_mapping_offset + frame.start_address().as_u64(),
            );
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
            match unsafe {
                MAPPER.get_unchecked().lock().map_to(
                    page,
                    frame,
                    flags,
                    &mut *FRAME_ALLOCATOR.get_unchecked().lock(),
                )
            } {
                Ok(tlb) => {
                    tlb.flush();
                    news += 1;
                }
                Err(_) => {
                    olds += 1;
                }
            };
        }
        // log::info!("new:{} already_mapped:{}", news, olds);
    }

    with_mapper_framealloc(|mapper, frame_allocator| {
        allocator::init_heap(mapper, frame_allocator).expect("heap initialization failed");
    });

    let rsdp_addr = boot_info.rsdp_addr.into_option().expect("no rsdp");
    let acpi_tables = unsafe { AcpiTables::from_rsdp(ACPI_HANDLER, rsdp_addr as usize).unwrap() };
    // log::info!("acpi_read");

    let _x = HpetInfo::new(&acpi_tables).expect("hpet");
    // log::info!("{:#?}]", x);

    let pi = acpi_tables.platform_info().expect("platform info");

    if let InterruptModel::Apic(apic) = pi.interrupt_model {
        // log::info!("{:#?}", apic);

        unsafe {
            // log::info!("init apic");
            let lapic = local_apic::LocalApic::init(PhysAddr::new(apic.local_apic_address));
            // log::info!("start apic c");
            let mut freq = 1000_000_000;
            if let Some(cpuid) = local_apic::cpuid() {
                // log::info!("cpuid");
                if let Some(tsc) = cpuid.get_tsc_info() {
                    // log::info!(
                    //     "{} {}",
                    //     tsc.nominal_frequency(),
                    //     tsc.tsc_frequency().unwrap()
                    // );
                    freq = tsc.nominal_frequency();
                } else {
                }
            }
            lapic.set_div_conf(0b1011);
            // log::info!("start apic c");
            lapic.set_lvt_timer((1 << 17) + 48);
            let wanted_freq_hz = 1000;
            lapic.set_init_count(freq / wanted_freq_hz);
        }

        for io_apic in apic.io_apics.iter() {
            // log::info!("{:x}", io_apic.address);
            let ioa = ioapic::IoApic::init(io_apic);
            let val = ioa.read(ioapic::IOAPICVER);
            // log::info!("{:x}", val);
            for i in 0..24 {
                let n = ioa.read_redtlb(i);
                let mut red = ioapic::RedTbl::new(n);
                red.vector = (50 + i) as u8;

                let stored = red.store();

                ioa.write_redtlb(i, stored);
            }
        }

        x86_64::instructions::interrupts::enable();

        // x86_64::instructions::interrupts::disable();
    }

    {
        // aml::AmlContext::new()
    }
    // .expect("no acpi table");

    let proc_info = pi.processor_info.expect("processor_info");
    // log::info!("{:?}", pi.power_profile);
    // log::info!("{:#?}", pi.interrupt_model);
    // log::info!("{:?}", proc_info.boot_processor);
    for proc in proc_info.application_processors.iter() {
        log::info!("{:?}", proc);
    }

    // for ent in mapper.level_4_table().iter().take(30) {
    //     log::info!("{:?}", ent);
    // }

    let pcis = pci::Pcis::new();

    let mut virtio_devices = Vec::new();

    {
        for (pci_index, pci) in pcis.devs.iter().enumerate() {
            let _vector_base = 50 + 2 * pci_index;
            let _status = pci.config_read_u16(pci::PCIConfigRegisters::PCIStatus as u8);
            let vendor = pci.config_read_u16(pci::PCIConfigRegisters::PCIVendorID as u8);
            let _device_id =
                pci.config_read_u16(pci::PCIConfigRegisters::PCIDeviceID as u8) as isize - 0x1040;
            // log::info!(
            //     "{:?} status {} irq:{} ipin:{}, {:x} {} ________________",
            //     pci,
            //     status,
            //     pci.get_irq(),
            //     pci.get_ipin(),
            //     vendor,
            //     device_id,
            // );
            const VIRTIO_VENDOR_ID: u16 = 0x1af4;
            if vendor == VIRTIO_VENDOR_ID {
                let virtio = with_mapper_framealloc(|mapper, frame_allocator| {
                    Virtio::init(pci, mapper, frame_allocator)
                });
                if let Some(virtio) = virtio {
                    virtio_devices.push(virtio);
                }
            }
        }
    }

    let mut fb = Box::new(FB::new(&fbinfo));
    let fb_clone: *mut FB = &mut *fb;
    // log::info!("fbclone {:?}", fb_clone);

    {
        let mut executor = task::executor::Executor::new();
        let spawner = executor.spawner();

        for virtio in virtio_devices.into_iter() {
            match virtio.device_type {
                DeviceType::Input => spawner.run(drivers::virtio_input::drive(virtio)),
                DeviceType::Gpu => spawner.run(drivers::virtio_gpu::drive(
                    virtio,
                    spawner.clone(),
                    fb_clone,
                )),
            }
        }

        spawner.run(async move {
            let mut apps: Vec<WasmApp> = Vec::new();
            let apps_raw = [
                &include_bytes!("../../../apps/test-app/target/wasm32-wasi/release/test_app.wasm")
                    [..],
                // &include_bytes!("../../../apps/zig-app/zig-out/lib/zig-app.wasm")[..],
                // &include_bytes!("../../../disk/bin/hello.wasm")[..],
                // &include_bytes!("../../../disk/bin/sqlite.wasm")[..],
            ];
            for app_bytes in apps_raw.iter() {
                apps.push(WasmApp::new(app_bytes.to_vec(), fb_clone));
            }

            for app in apps.iter_mut() {
                app.call();
            }

            loop {
                globals::INPUT.update(|e| e.step());
                let input = globals::INPUT.read();
                for app in apps.iter_mut() {
                    app.call_update(input);
                }
                yield_once().await;
            }
        });
        executor.run();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::error!("{:?}", info);
    loop {}
}
