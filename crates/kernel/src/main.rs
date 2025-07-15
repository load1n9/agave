#![no_std]
#![no_main]



use acpi::{AcpiTables, HpetInfo, InterruptModel};
use agave_api::sys::{
    allocator, 
    diagnostics,    // New: Enhanced diagnostics
    drivers,
    framebuffer::{FB, RGBA},
    fs,        // Add filesystem
    gdt, globals, interrupts, ioapic, local_apic,
    logger::init_logger,
    memory::{self, BootInfoFrameAllocator},
    monitor,
    network,   // Add network
    pci,
    power,     // New: Power management
    process,   // New: Process management
    security,  // New: Security framework
    task::{self, executor::yield_once},
    virtio::{DeviceType, Virtio},
    wasm::WasmApp,
    with_mapper_framealloc, ACPI_HANDLER, FRAME_ALLOCATOR, MAPPER, VIRTUAL_MAPPING_OFFSET,
};
use alloc::{boxed::Box, vec::Vec};
use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use bootloader_boot_config::LevelFilter;
use core::panic::PanicInfo;
use spin::Mutex;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use x86_64::{
    structures::paging::{Mapper, Page, PageTableFlags, PhysFrame, Size1GiB, Size2MiB},
    PhysAddr, VirtAddr,
};
extern crate agave_api;
extern crate alloc;

// Entry point configuration
const CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config.kernel_stack_size = 128 * 1024;
    config
};
entry_point!(main, config = &CONFIG);

fn main(boot_info: &'static mut BootInfo) -> ! {
    // Initialize framebuffer and logger FIRST
    let framebuffer = boot_info.framebuffer.as_mut().unwrap();
    let fbinfo = framebuffer.info();
    let fbm = framebuffer.buffer_mut();
    // let fbm2 = unsafe {
    //     let p = fbm.as_mut_ptr();
    //     slice::from_raw_parts_mut(p, fbinfo.byte_len)
    // };

    init_logger(fbm, fbinfo.clone(), LevelFilter::Trace, true, true);
    log::info!("KERNEL: Starting main() function - logger initialized");
    
    // Now initialize GDT and IDT
    log::info!("KERNEL: Initializing GDT");
    gdt::init();
    log::info!("KERNEL: Initializing IDT");
    interrupts::init_idt();
    log::info!("KERNEL: Basic initialization complete");
    
    // TODO: Add loading screen once framebuffer is accessible
    // show_loading_screen("Initializing Memory Management...", 10, &mut fb);

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

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
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
        log::info!("new:{} already_mapped:{}", news, olds);
    }

    with_mapper_framealloc(|mapper, frame_allocator| {
        match allocator::init_heap(mapper, frame_allocator) {
            Ok(()) => log::info!("Heap initialization successful"),
            Err(e) => {
                log::error!("Heap initialization failed: {}", e);
                panic!("Failed to initialize heap");
            }
        }
    });

    let rsdp_addr = boot_info.rsdp_addr.into_option().expect("no rsdp");
    let acpi_tables = unsafe { AcpiTables::from_rsdp(ACPI_HANDLER, rsdp_addr as usize).unwrap() };
    // log::info!("acpi_read");

    let x = HpetInfo::new(&acpi_tables).expect("hpet");
    log::info!("{:#?}]", x);

    let pi = acpi_tables.platform_info().expect("platform info");
    log::info!("ACPI platform info obtained");

    if let InterruptModel::Apic(apic) = pi.interrupt_model {
        log::info!("Setting up APIC interrupts...");
        // log::info!("{:#?}", apic);

        unsafe {
            log::info!("Initializing local APIC...");
            // log::info!("init apic");
            let lapic = local_apic::LocalApic::init(PhysAddr::new(apic.local_apic_address));
            log::info!("Local APIC initialized");
            // log::info!("start apic c");
            let mut freq = 1000_000_000;
            if let Some(cpuid) = local_apic::cpuid() {
                log::info!("CPUID info obtained");
                // log::info!("cpuid");
                if let Some(tsc) = cpuid.get_tsc_info() {
                    log::info!(
                        "{} {}",
                        tsc.nominal_frequency(),
                        tsc.tsc_frequency().unwrap()
                    );
                    freq = tsc.nominal_frequency();
                } else {
                }
            }
            log::info!("Setting APIC timer configuration...");
            lapic.set_div_conf(0b1011);
            log::info!("start apic c");
            lapic.set_lvt_timer((1 << 17) + 48);
            let wanted_freq_hz = 1000;
            lapic.set_init_count(freq / wanted_freq_hz);
            log::info!("APIC timer configured");
        }

        log::info!("Setting up IO APICs...");
        for io_apic in apic.io_apics.iter() {
            log::info!("{:x}", io_apic.address);
            let ioa = ioapic::IoApic::init(io_apic);
            let val = ioa.read(ioapic::IOAPICVER);
            log::info!("{:x}", val);
            for i in 0..24 {
                let n = ioa.read_redtlb(i);
                let mut red = ioapic::RedTbl::new(n);
                red.vector = (50 + i) as u8;

                let stored = red.store();

                ioa.write_redtlb(i, stored);
            }
        }
        log::info!("IO APICs configured");

        log::info!("Enabling interrupts...");
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        x86_64::instructions::interrupts::enable();
        log::info!("Interrupts enabled");

        // #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        // x86_64::instructions::interrupts::disable();
    }

    log::info!("APIC setup complete, proceeding to PCI discovery...");

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

    log::info!("Starting PCI device discovery...");
    let pcis = pci::Pcis::new();
    log::info!("Found {} PCI devices", pcis.devs.len());

    let mut virtio_devices = Vec::new();

    {
        log::info!("Scanning for VirtIO devices...");
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
                log::info!("Found VirtIO device at PCI index {}", pci_index);
                let virtio = with_mapper_framealloc(|mapper, frame_allocator| {
                    Virtio::init(pci, mapper, frame_allocator)
                });
                if let Some(virtio) = virtio {
                    log::info!("VirtIO device initialized successfully");
                    virtio_devices.push(virtio);
                } else {
                    log::warn!("Failed to initialize VirtIO device");
                }
            }
        }
        log::info!("VirtIO device scan complete, found {} devices", virtio_devices.len());
    }

    log::info!("Setting up framebuffer...");

    log::info!("Setting up framebuffer...");
    let mut fb = Box::new(FB::new(&fbinfo));
    let fb_clone: *mut FB = &mut *fb;
    log::info!("Framebuffer created at {:?}", fb_clone);
    
    // Show loading screen now that framebuffer is available
    show_loading_screen("Basic initialization complete...", 25, &mut *fb);
    
    // log::info!("fbclone {:?}", fb_clone);

    // Initialize monitoring system
    log::info!("Initializing system monitoring...");
    monitor::init_monitoring();
    log::info!("System monitoring enabled");
    show_loading_screen("System monitoring enabled...", 35, &mut *fb);

    // Initialize enhanced diagnostics
    log::info!("Initializing enhanced diagnostics...");
    diagnostics::init_diagnostics();
    log::info!("Enhanced diagnostics enabled");
    show_loading_screen("Enhanced diagnostics enabled...", 40, &mut *fb);

    // Initialize security framework
    log::info!("Initializing security framework...");
    security::init_security();
    log::info!("Security framework enabled");
    show_loading_screen("Security framework enabled...", 45, &mut *fb);

    // Initialize process management
    log::info!("Initializing process management...");
    process::init_process_management();
    log::info!("Process management enabled");
    show_loading_screen("Process management enabled...", 50, &mut *fb);

    // Initialize power management
    log::info!("Initializing power management...");
    power::init_power_management();
    log::info!("Power management enabled");
    show_loading_screen("Power management enabled...", 55, &mut *fb);

    // Initialize filesystem
    log::info!("Initializing filesystem...");
    if let Err(e) = fs::init_filesystem() {
        log::error!("Failed to initialize filesystem: {:?}", e);
    } else {
        log::info!("Filesystem initialized successfully");
    }
    show_loading_screen("Filesystem initialized...", 65, &mut *fb);

    // Initialize network stack
    log::info!("Initializing network stack...");
    if let Err(e) = network::init_network() {
        log::error!("Failed to initialize network: {:?}", e);
    } else {
        log::info!("Network stack initialized successfully");
    }
    show_loading_screen("Network stack ready...", 75, &mut *fb);

    // Initialize socket subsystem
    log::info!("Initializing socket subsystem...");
    if let Err(e) = network::sockets::init_sockets() {
        log::error!("Failed to initialize sockets: {:?}", e);
    } else {
        log::info!("Socket subsystem initialized successfully");
    }
    show_loading_screen("Socket subsystem ready...", 80, &mut *fb);

    // Log initial system status
    log::info!("Logging initial system status...");
    monitor::log_system_status();

    log::info!("Setting up task executor...");
    {
        let mut executor = task::executor::Executor::new();
        let spawner = executor.spawner();
        log::info!("Task executor created");
        show_loading_screen("Task executor ready...", 95, &mut *fb);

        log::info!("Setting up VirtIO device drivers...");
        for virtio in virtio_devices.into_iter() {
            match virtio.device_type {
                DeviceType::Input => {
                    log::info!("Spawning VirtIO input driver task");
                    spawner.run(drivers::virtio_input::drive(virtio));
                }
                DeviceType::Gpu => {
                    log::info!("Spawning VirtIO GPU driver task");
                    spawner.run(drivers::virtio_gpu::drive(
                        virtio,
                        spawner.clone(),
                        fb_clone,
                    ));
                }
                DeviceType::Network => {
                    log::info!("Spawning VirtIO network driver task");
                    spawner.run(drivers::virtio_net::drive(virtio));
                }
                DeviceType::Block => {
                    log::info!("VirtIO block device detected - driver not yet implemented");
                    // TODO: Implement VirtIO block driver
                }
                DeviceType::Console => {
                    log::info!("VirtIO console device detected - driver not yet implemented");
                    // TODO: Implement VirtIO console driver
                }
                DeviceType::Balloon => {
                    log::info!("VirtIO balloon device detected - driver not yet implemented");
                    // TODO: Implement VirtIO balloon driver
                }
                DeviceType::Scsi => {
                    log::info!("VirtIO SCSI device detected - driver not yet implemented");
                    // TODO: Implement VirtIO SCSI driver
                }
                DeviceType::Unknown(id) => {
                    log::warn!("Unknown VirtIO device type {} detected - no driver available", id);
                }
            }
        }
        log::info!("VirtIO drivers spawned");

        log::info!("Setting up WASM application task...");
        spawner.run(async move {
            log::info!("WASM task started - loading applications...");
            let mut apps: Vec<WasmApp> = Vec::new();
            let apps_raw = [
                &include_bytes!("../../../apps/terminal/target/wasm32-wasip1/release/terminal_app.wasm")
                    [..],
                // &include_bytes!("../../../apps/zig-app/zig-out/lib/zig-app.wasm")[..],
                // &include_bytes!("../../../disk/bin/hello.wasm")[..],
                // &include_bytes!("../../../disk/bin/sqlite.wasm")[..],
            ];
            log::info!("Creating WASM app instances...");
            for app_bytes in apps_raw.iter() {
                log::info!("Creating WASM app from {} bytes", app_bytes.len());
                apps.push(WasmApp::new(app_bytes.to_vec(), fb_clone));
            }
            log::info!("Created {} WASM apps", apps.len());

            log::info!("Initializing WASM applications...");
            for app in apps.iter_mut() {
                log::info!("Calling WASM app initialization...");
                app.call();
            }
            log::info!("WASM apps initialized");

            let mut frame_counter = 0u64;
            let mut last_monitor_check = 0u64;
            
            // Show completion screen - access framebuffer through the raw pointer
            unsafe {
                let fb_ref = &mut *fb_clone;
                show_loading_screen("Agave OS Ready!", 100, fb_ref);
                // Give the user a moment to see the completion
                for _ in 0..1000000 { 
                    core::hint::spin_loop(); 
                }
            }
            
            log::info!("Starting main application loop...");
            loop {
                globals::INPUT.update(|e| e.step());
                let input = globals::INPUT.read();
                
                // Record system activity for power management
                power::record_system_activity();
                
                for app in apps.iter_mut() {
                    app.call_update(input);
                }
                
                frame_counter += 1;
                
                // Update power management every 10 frames (~100Hz)
                if frame_counter % 10 == 0 {
                    if let Err(e) = power::update_power_management() {
                        log::error!("Power management update failed: {:?}", e);
                    }
                }
                
                // Enhanced monitoring and diagnostics (every ~1000 frames, roughly once per second)
                if frame_counter % 1000 == 0 {
                    let current_time = agave_api::sys::interrupts::TIME_MS
                        .load(core::sync::atomic::Ordering::Relaxed);
                    
                    // Run periodic diagnostics every 10 seconds
                    if current_time - last_monitor_check > 10000 {
                        let health_report = diagnostics::perform_health_check();
                        
                        // Log critical issues immediately
                        if health_report.status == diagnostics::SystemHealthStatus::Critical {
                            log::error!("CRITICAL SYSTEM HEALTH ISSUE DETECTED!");
                            log::error!("{}", health_report.summary());
                        }
                        
                        // Run legacy monitoring
                        monitor::periodic_monitor_check();
                        
                        // Clean up terminated processes
                        process::cleanup_terminated_processes();
                        
                        last_monitor_check = current_time;
                    }
                    
                    // Log comprehensive system status every 30 seconds
                    if frame_counter % 30000 == 0 {
                        monitor::log_system_status();
                        
                        // Log power status
                        let (cpu_freq, _, _) = power::get_cpu_frequency_info();
                        let (cpu_temp, thermal_throttling) = power::get_thermal_info();
                        let power_state = power::get_power_state();
                        
                        log::info!("Enhanced Status - Power: {:?}, CPU: {} MHz, Temp: {:.1}°C, Throttling: {}",
                                   power_state, cpu_freq, cpu_temp, thermal_throttling);
                        
                        // Log security status
                        let security_stats = security::get_security_statistics();
                        if security_stats.total_events > 0 {
                            log::info!("Security Status - {} events, {} blocked processes",
                                       security_stats.total_events, security_stats.blocked_processes);
                        }
                    }
                }
                
                yield_once().await;
            }
        });
        log::info!("WASM task spawned, starting executor...");
        executor.run();
    }
}

/// Display a loading screen with progress indicators
fn show_loading_screen(_stage: &str, progress: u8, fb: &mut FB) {
    // Get screen dimensions
    let width = fb.w as i32;
    let height = fb.h as i32;
    
    // Fill background with dark blue
    let bg_color = RGBA { r: 0x1a, g: 0x1a, b: 0x2e, a: 0xFF };
    for y in 0..height {
        for x in 0..width {
            fb.set(x as usize, y as usize, bg_color);
        }
    }
    
    // Draw progress bar background
    let bar_width = 400;
    let bar_height = 20;
    let bar_x = (width - bar_width) / 2;
    let bar_y = height / 2 + 50;
    
    let bar_bg_color = RGBA { r: 0x0f, g: 0x0f, b: 0x23, a: 0xFF };
    for y in bar_y..(bar_y + bar_height) {
        for x in bar_x..(bar_x + bar_width) {
            if x >= 0 && x < width && y >= 0 && y < height {
                fb.set(x as usize, y as usize, bar_bg_color);
            }
        }
    }
    
    // Draw progress bar fill
    let fill_width = (bar_width as u8 * progress / 100) as i32;
    if fill_width > 0 {
        let fill_color = RGBA { r: 0x00, g: 0xd4, b: 0xaa, a: 0xFF };
        for y in bar_y..(bar_y + bar_height) {
            for x in bar_x..(bar_x + fill_width) {
                if x >= 0 && x < width && y >= 0 && y < height {
                    fb.set(x as usize, y as usize, fill_color);
                }
            }
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Disable interrupts to prevent further issues
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    x86_64::instructions::interrupts::disable();
    
    log::error!("=== KERNEL PANIC ===");
    log::error!("Panic info: {:?}", info);
    
    // Log system state at panic
    log::error!("System uptime: {}ms", 
        agave_api::sys::interrupts::TIME_MS.load(core::sync::atomic::Ordering::Relaxed));
    
    // Log memory state
    let memory_stats = agave_api::sys::allocator::memory_stats();
    log::error!("Memory usage: {}/{} bytes ({:.1}%)", 
        memory_stats.allocated, 
        memory_stats.heap_size,
        memory_stats.utilization_percent());
    
    // Log task metrics
    {
        let task_metrics = agave_api::sys::task::executor::TASK_METRICS.lock();
        log::error!("Tasks: {} spawned, {} completed, {} context switches",
            task_metrics.total_tasks_spawned,
            task_metrics.tasks_completed,
            task_metrics.context_switches);
    }
    
    log::error!("System halted due to panic");
    
    loop {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        x86_64::instructions::hlt();
    }
}

