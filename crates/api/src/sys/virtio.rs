use crate::sys::{
    create_identity_virt_from_phys,
    pci::{self, Bar, Pci},
    phys_to_virt,
};
use alloc::{fmt, vec::Vec};
use core::ptr::{read_volatile, write_volatile};
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use x86_64::{
    structures::paging::{FrameAllocator, Mapper, Size4KiB},
    PhysAddr, VirtAddr,
};

// VirtIO Constants
const MAX_NUM_QUEUE: usize = 256;
const DEVICE_ID_INPUT: isize = 18;
const DEVICE_ID_GPU: isize = 16;
const DEVICE_ID_NETWORK: isize = 1;
const DEVICE_ID_BLOCK: isize = 2;
const DEVICE_ID_CONSOLE: isize = 3;
const DEVICE_ID_BALLOON: isize = 5;
const DEVICE_ID_SCSI: isize = 8;

// VirtIO PCI Capability Types
const VIRTIO_PCI_CAP_COMMON_CFG: u8 = 1;
const VIRTIO_PCI_CAP_NOTIFY_CFG: u8 = 2;
const VIRTIO_PCI_CAP_ISR_CFG: u8 = 3;
const VIRTIO_PCI_CAP_DEVICE_CFG: u8 = 4;
const VIRTIO_PCI_CAP_PCI_CFG: u8 = 5;

// VirtIO Queue Descriptor Flags
const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;
const VIRTQ_DESC_F_INDIRECT: u16 = 4;

// VirtIO Device Status
const VIRTIO_STATUS_ACKNOWLEDGE: u8 = 1;
const VIRTIO_STATUS_DRIVER: u8 = 2;
const VIRTIO_STATUS_DRIVER_OK: u8 = 4;
const VIRTIO_STATUS_FEATURE_OK: u8 = 8;
const VIRTIO_STATUS_DEVICE_NEEDS_RESET: u8 = 64;
const VIRTIO_STATUS_FAILED: u8 = 128;

// VirtIO Feature bits (common)
const VIRTIO_F_RING_INDIRECT_DESC: u64 = 1 << 28;
const VIRTIO_F_RING_EVENT_IDX: u64 = 1 << 29;
const VIRTIO_F_VERSION_1: u64 = 1 << 32;
const VIRTIO_F_ACCESS_PLATFORM: u64 = 1 << 33;
const VIRTIO_F_RING_PACKED: u64 = 1 << 34;

pub fn to_bytes<T>(t: &T) -> &[u8] {
    unsafe {
        let len = core::intrinsics::size_of_val(t);
        let ptr: *const u8 = core::intrinsics::transmute(t);
        core::slice::from_raw_parts(ptr, len)
    }
}

pub struct Virtio {
    pub pci: Pci,
    pub common: VirtioCap<&'static mut VirtioPciCommonCfg>,
    pub device: VirtioCap<&'static mut ()>,
    pub notify: VirtioCap<u32>,
    pub pci_conf: VirtioCap<[u8; 4]>,
    pub isr: Option<VirtioCap<&'static mut u8>>,
    
    pub step: usize,
    pub device_type: DeviceType,
    pub queues: Vec<VirtQueue>,
    pub queue_select: u16,
    
    // Feature negotiation
    pub device_features: u64,
    pub driver_features: u64,
    pub feature_select: u32,
    
    // Device status
    pub status: u8,
    
    // Configuration generation
    pub config_generation: u8,
}

#[derive(Debug)]
pub struct QueueFreeDescs {
    free: Vec<u16>,
}

impl QueueFreeDescs {
    pub fn new(queue_size: u16) -> Self {
        let mut free = Vec::with_capacity(queue_size as usize);
        for i in 0..queue_size {
            free.push(i as u16);
        }
        Self { free }
    }

    pub fn get_free(&mut self) -> Option<u16> {
        self.free.pop()
    }

    pub fn get_free_twice(&mut self) -> Option<(u16, u16)> {
        if self.free.len() >= 2 {
            Some((self.free.pop().unwrap(), self.free.pop().unwrap()))
        } else {
            None
        }
    }

    pub fn set_free(&mut self, desc_id: u16) {
        self.free.push(desc_id);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeviceType {
    Input,
    Gpu,
    Network,
    Block,
    Console,
    Balloon,
    Scsi,
    Unknown(isize),
}

fn device_id_to_type(id: isize) -> DeviceType {
    match id {
        DEVICE_ID_INPUT => DeviceType::Input,
        DEVICE_ID_GPU => DeviceType::Gpu,
        DEVICE_ID_NETWORK => DeviceType::Network,
        DEVICE_ID_BLOCK => DeviceType::Block,
        DEVICE_ID_CONSOLE => DeviceType::Console,
        DEVICE_ID_BALLOON => DeviceType::Balloon,
        DEVICE_ID_SCSI => DeviceType::Scsi,
        _ => DeviceType::Unknown(id),
    }
}

/// VirtIO Queue management with enhanced features
#[derive(Debug)]
pub struct VirtQueue {
    pub desc: VirtAddr,
    pub driver: VirtAddr,
    pub device: VirtAddr,
    pub size: u16,
    pub notify_off: u16,
    pub msix_vector: u16,
    pub enabled: bool,
    pub free_descriptors: QueueFreeDescs,
    pub last_used_idx: u16,
    pub features: u64,
}

impl VirtQueue {
    pub fn new(size: u16) -> Self {
        Self {
            desc: VirtAddr::new(0),
            driver: VirtAddr::new(0),
            device: VirtAddr::new(0),
            size,
            notify_off: 0,
            msix_vector: 0,
            enabled: false,
            free_descriptors: QueueFreeDescs::new(size),
            last_used_idx: u16::MAX,
            features: 0,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn get_free_descriptor(&mut self) -> Option<u16> {
        self.free_descriptors.get_free()
    }

    pub fn return_descriptor(&mut self, desc_id: u16) {
        self.free_descriptors.set_free(desc_id);
    }
}

impl Virtio {
    pub fn init(
        pci: &Pci,
        mapper: &mut impl Mapper<Size4KiB>,
        frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    ) -> Option<Self> {
        let device_id =
            pci.config_read_u16(pci::PCIConfigRegisters::PCIDeviceID as u8) as isize - 0x1040;

        let device_type = device_id_to_type(device_id);
        
        // Only proceed with known device types for now
        if matches!(device_type, DeviceType::Unknown(_)) {
            return None;
        }

        let mut bars = [Bar::None; 6];
        for idx in 0..=5 {
            let bar = pci.get_bar(idx);
            if bar != Bar::None {
                // log::info!("bar {}:{:?}", idx, bar);
            }
            bars[idx as usize] = bar;
        }

        let cap_ptr = pci.config_read_u8(pci::PCIConfigRegisters::PCICapabilitiesPointer as u8);

        let mut current_off = cap_ptr;
        #[allow(dead_code)]
        const VIRTIO_STATUS_NONE: u8 = 0;
        const VIRTIO_STATUS_ACKNOWLEDGE: u8 = 1;
        #[allow(dead_code)]
        const VIRTIO_STATUS_DRIVER: u8 = 2;
        #[allow(dead_code)]
        const VIRTIO_STATUS_FAILED: u8 = 128;
        const VIRTIO_STATUS_FEATURE_OK: u8 = 8;
        const VIRTIO_STATUS_DRIVER_OK: u8 = 4;
        #[allow(dead_code)]
        const VIRTIO_STATUS_NEEDS_RESET: u8 = 64;

        let mut common: Option<VirtioCap<&'static mut VirtioPciCommonCfg>> = None;
        let mut device: Option<VirtioCap<&'static mut ()>> = None;
        let mut notify: Option<VirtioCap<u32>> = None;
        let mut pci_conf: Option<VirtioCap<[u8; 4]>> = None;
        loop {
            let cap = pci.config_read_u8(current_off);

            //VIRTIO
            if cap == 0x9 {
                let _cap_len = pci.config_read_u8(current_off + 2);
                let cfg_type = pci.config_read_u8(current_off + 3);
                let bar = pci.config_read_u8(current_off + 4);
                let offset = pci.config_read_u32(current_off + 8);
                let length = pci.config_read_u32(current_off + 12);

                // log::info!(
                //     "virtio {} {} {} {} {}",
                //     cap_len,
                //     cfg_type,
                //     bar,
                //     offset,
                //     length
                // );

                match cfg_type {
                    #[allow(unused_variables)]
                    VIRTIO_PCI_CAP_COMMON_CFG => {
                        if let Bar::Mm(phys) = bars[bar as usize] {
                            let ptr = phys_to_virt(phys + offset as u64);
                            // log::info!("common {:?}", ptr.as_ptr::<VirtioPciCommonCfg>());
                            let cfg: &'static mut VirtioPciCommonCfg =
                                unsafe { &mut *(ptr.as_mut_ptr::<VirtioPciCommonCfg>()) };

                            let x: u8 = unsafe { read_volatile(ptr.as_ptr()) };
                            // log::info!("{}", x);
                            // log::info!("common {:?}", cfg);
                            common = Some(VirtioCap::new(cfg, bars[bar as usize], offset, length));
                        }
                    }
                    VIRTIO_PCI_CAP_NOTIFY_CFG => {
                        let notify_off_multiplier = pci.config_read_u32(current_off + 16);
                        notify = Some(VirtioCap {
                            cap: notify_off_multiplier,
                            bar: bars[bar as usize],
                            offset,
                            length,
                        });
                        // log::info!("{:#?}", virtio_caps.notify);
                    }
                    VIRTIO_PCI_CAP_ISR_CFG => {}
                    VIRTIO_PCI_CAP_DEVICE_CFG => {
                        if let Bar::Mm(phys) = bars[bar as usize] {
                            let ptr: VirtAddr = phys_to_virt(phys + offset as u64);
                            let cfg: &'static mut () = unsafe { &mut *(ptr.as_mut_ptr::<()>()) };
                            device = Some(VirtioCap {
                                cap: cfg,
                                bar: bars[bar as usize],
                                offset,
                                length,
                            });
                        }
                    }
                    VIRTIO_PCI_CAP_PCI_CFG => {
                        let pci_cfg_data = [
                            pci.config_read_u8(current_off + 16),
                            pci.config_read_u8(current_off + 17),
                            pci.config_read_u8(current_off + 18),
                            pci.config_read_u8(current_off + 19),
                        ];

                        pci_conf = Some(VirtioCap {
                            cap: pci_cfg_data,
                            bar: bars[bar as usize],
                            offset,
                            length,
                        });
                    }
                    _ => {}
                }
            }
            // if cap == 0x11 && false {
            //     let line2 = pci.config_read_u32(current_off + 4);
            //     let table_bir = line2 & 0b111;
            //     let table_offset = line2 & 0xFFFFFFf8;
            //     let msg_ctrl = pci.config_read_u16(current_off + 2);
            //     bitfield! {
            //       pub struct MsgCtrl(u16);
            //       impl Debug;
            //       // The fields default to u16
            //       pub table_size, _: 10, 0;
            //       pub reserved, _ : 13, 11;
            //       pub function_mask , _: 14;
            //       pub enable , set_enable: 15;
            //     }

            //     let mut msg_ctrl = MsgCtrl(msg_ctrl);
            //     msg_ctrl.set_enable(true);

            //     pci.config_write_u16(current_off + 2, msg_ctrl.0);
            //     log::info!(
            //         "MSI-X bir:{} tblo:{} msg_ctrl:{:?}",
            //         table_bir,
            //         table_offset,
            //         msg_ctrl
            //     );

            //     let line3 = pci.config_read_u32(current_off + 8);
            //     let pba_bir = line3 & 0b11;
            //     let pba_offset = line3 & 0xFFFFFFf8;

            //     for table_index in 0..=msg_ctrl.table_size() {
            //         if let Bar::Mm(phys) = bars[table_bir as usize] {
            //             let virt_bar = create_virt_from_phys(
            //                 &mut mapper,
            //                 &mut frame_allocator,
            //                 PhysFrame::containing_address(phys),
            //             )
            //             .expect("bar");

            //             // let virt_bar = create_virt_from_phys(
            //             //     &mut mapper,
            //             //     &mut frame_allocator,
            //             //     PhysFrame::containing_address(PhysAddr::new(bars[table_bir as usize])),
            //             // )
            //             // .expect("bar");

            //             let ptr: VirtAddr = virt_bar.start_address()
            //                 + (table_offset as u64)
            //                 + (table_index as u64) * 16;

            //             use core::intrinsics::{volatile_load, volatile_store};

            //             let table = unsafe { volatile_load(ptr.as_ptr() as *const u128) };

            //             bitfield! {
            //               pub struct TableEntry(u128);
            //               impl Debug;
            //               // The fields default to u16
            //               u64, address, set_address : 63, 0;
            //               pub data, set_data : 95, 64;
            //               pub mask, set_mask : 96;
            //               pub reserved , _: 127, 97;

            //             }

            //             let mut table = TableEntry(table);
            //             log::info!("{:?}", table);
            //             table.set_data(vector_base as u128 + table_index as u128);

            //             table.set_address(0xFEE00000 + (0 << 12));
            //             table.set_mask(false);
            //             unsafe { volatile_store(ptr.as_mut_ptr() as *mut u128, table.0) };
            //             // log::info!("{:?}", table);

            //             //READ PENDING
            //             // {
            //             //     let virt_bar = create_virt_from_phys(
            //             //         &mut mapper,
            //             //         &mut frame_allocator,
            //             //         PhysFrame::containing_address(PhysAddr::new(
            //             //             bars[pba_bir as usize],
            //             //         )),
            //             //     )
            //             //     .expect("bar");

            //             //     let ptr: VirtAddr = virt_bar.start_address()
            //             //         + (pba_offset as u64)
            //             //         + (table_index as u64) * 2;

            //             //     let table = unsafe { volatile_load(ptr.as_ptr() as *const u64) };
            //             //     if table != 0 {
            //             //         log::info!("pending {:b}", table);
            //             //     }
            //             // }
            //         }

            //         let command = pci.config_read_u16(4);
            //         log::info!("com {:b}", command);
            //         pci.config_write_u16(4, command | 0b11);

            //         unsafe {
            //             crate::local_apic::LocalApic.get().unwrap().eoi();
            //         };
            //     }
            // }

            current_off = pci.config_read_u8(1 + current_off);

            if current_off == 0 {
                break;
            }
            // break;
        }

        // log::info!("virtio_caps {:?}", virtio_caps);

        if common.is_none() || pci_conf.is_none() || device.is_none() || notify.is_none() {
            return None;
        }

        let mut common = common.unwrap();
        let pci_conf = pci_conf.unwrap();
        let mut device = device.unwrap();
        let notify = notify.unwrap();

        let cap_common = &mut common.cap;

        unsafe {
            let mut queues = Vec::new();
            write_volatile(&mut cap_common.device_status, 0);

            write_volatile(
                &mut cap_common.device_status,
                read_volatile(&cap_common.device_status) | VIRTIO_STATUS_ACKNOWLEDGE,
            );

            write_volatile(
                &mut cap_common.device_status,
                read_volatile(&cap_common.device_status) | VIRTIO_STATUS_DRIVER,
            );

            let _current = read_volatile(*cap_common);
            // Feature negotiation
            match device_type {
                DeviceType::Gpu => {
                    write_volatile(&mut cap_common.driver_feature, 0b11);
                }
                DeviceType::Network => {
                    // Enable network-specific features
                    write_volatile(&mut cap_common.driver_feature, 0);
                }
                DeviceType::Input => {
                    write_volatile(&mut cap_common.driver_feature, 0);
                }
                DeviceType::Block | DeviceType::Console | DeviceType::Balloon | 
                DeviceType::Scsi | DeviceType::Unknown(_) => {
                    write_volatile(&mut cap_common.driver_feature, 0);
                }
            }

            write_volatile(
                &mut cap_common.device_status,
                read_volatile(&cap_common.device_status) | VIRTIO_STATUS_FEATURE_OK,
            );

            if read_volatile(&cap_common.device_status) & VIRTIO_STATUS_FEATURE_OK == 0 {
                panic!("Cant enable set of feature")
            }

            // Initialize queues
            for q in 0..cap_common.num_queues {
                write_volatile(&mut cap_common.queue_select, q);
                
                let queue_size = read_volatile(*cap_common).queue_size;
                let mut virt_queue = VirtQueue::new(queue_size);

                // Set up queue descriptors
                let desc_addr = create_identity_virt_from_phys(mapper, frame_allocator)
                    .unwrap()
                    .start_address()
                    .as_u64();
                write_volatile(&mut cap_common.queue_desc, desc_addr);
                virt_queue.desc = VirtAddr::new(desc_addr);

                // Initialize descriptors
                {
                    let descs = cap_common.queue_desc as *mut Desc;
                    let qsize = queue_size as isize;
                    for idesc in 0..qsize {
                        let elem_ptr = descs.offset(idesc);
                        elem_ptr.write_volatile(Desc {
                            addr: create_identity_virt_from_phys(mapper, frame_allocator)
                                .unwrap()
                                .start_address()
                                .as_u64(),
                            flags: VIRTQ_DESC_F_WRITE,
                            len: 4096,
                            next: 0xffff,
                        });
                    }
                }

                // Set up driver ring
                let driver_addr = create_identity_virt_from_phys(mapper, frame_allocator)
                    .unwrap()
                    .start_address()
                    .as_u64();
                write_volatile(&mut cap_common.queue_driver, driver_addr);
                virt_queue.driver = VirtAddr::new(driver_addr);

                // Disable device to driver notification (Interrupt)
                (cap_common.queue_driver as *mut u16).write_volatile(1);

                // Set up device ring
                let device_addr = create_identity_virt_from_phys(mapper, frame_allocator)
                    .unwrap()
                    .start_address()
                    .as_u64();
                write_volatile(&mut cap_common.queue_device, device_addr);
                virt_queue.device = VirtAddr::new(device_addr);

                // Enable queue
                write_volatile(&mut cap_common.queue_enable, 1);
                virt_queue.enabled = true;
                virt_queue.notify_off = read_volatile(*cap_common).queue_notify_off;

                queues.push(virt_queue);
            }

            let cap_device = &mut device.cap;

            match device_type {
                DeviceType::Input => {
                    let conf_ptr: *mut VirtioInputConfig =
                        core::intrinsics::transmute((*cap_device) as *mut ());
                    let _rconf = read_volatile(conf_ptr);
                    let conf: &mut VirtioInputConfig = conf_ptr.as_mut().unwrap();
                    write_volatile(&mut conf.select, 1);
                    let u = read_volatile(&conf.u);
                    log::info!(
                        "name: {:?}",
                        alloc::str::from_utf8_unchecked(
                            &u.bitmap[0..read_volatile(&conf.size) as usize]
                        )
                    );
                    write_volatile(&mut conf.select, 0);
                }
                DeviceType::Gpu => {
                    #[repr(C)]
                    #[derive(Clone, Debug)]
                    struct VirtioGpuConfig {
                        events_read: u32,
                        events_clear: u32,
                        num_scanouts: u32,
                        num_capsets: u32,
                    }
                    let conf_ptr: *mut VirtioGpuConfig =
                        core::intrinsics::transmute((*cap_device) as *mut ());
                    let _rconf = conf_ptr.read_volatile();
                    // log::info!("{:?}", rconf);
                    // rconf.events_clear = 1;
                    // conf_ptr.write_volatile(rconf);
                }
                DeviceType::Network => {
                    log::info!("VirtIO network device configuration");
                    // Network device configuration will be handled by the driver
                }
                DeviceType::Block => {
                    log::info!("VirtIO block device configuration");
                    // Block device configuration
                }
                DeviceType::Console => {
                    log::info!("VirtIO console device configuration");
                    // Console device configuration
                }
                DeviceType::Balloon => {
                    log::info!("VirtIO balloon device configuration");
                    // Balloon device configuration
                }
                DeviceType::Scsi => {
                    log::info!("VirtIO SCSI device configuration");
                    // SCSI device configuration
                }
                DeviceType::Unknown(id) => {
                    log::warn!("Unknown VirtIO device type: {}", id);
                }
            }

            write_volatile(
                &mut cap_common.device_status,
                read_volatile(&cap_common.device_status) | VIRTIO_STATUS_DRIVER_OK,
            );

            // Store values before moving common
            let device_features = read_volatile(&cap_common.device_feature) as u64;
            let driver_features = read_volatile(&cap_common.driver_feature) as u64;
            let status = read_volatile(&cap_common.device_status);
            let config_generation = read_volatile(&cap_common.config_generation);

            let mut this = Self {
                pci: pci.clone(),
                step: 0,
                device_type,
                queues,
                common,
                device,
                notify,
                pci_conf,
                isr: None, // Will be set up later if needed
                queue_select: 0,
                device_features,
                driver_features,
                feature_select: 0,
                status,
                config_generation,
            };
            this.queue_select(0);
            Some(this)
        }
    }

    pub fn get_free_desc_id(&mut self) -> Option<u16> {
        if let Some(queue) = self.queues.get_mut(self.queue_select as usize) {
            queue.get_free_descriptor()
        } else {
            None
        }
    }

    pub fn get_free_twice_desc_id(&mut self) -> Option<(u16, u16)> {
        if let Some(queue) = self.queues.get_mut(self.queue_select as usize) {
            let first = queue.get_free_descriptor()?;
            let second = queue.get_free_descriptor()?;
            Some((first, second))
        } else {
            None
        }
    }

    pub fn set_free_desc_id(&mut self, desc_id: u16) {
        if let Some(queue) = self.queues.get_mut(self.queue_select as usize) {
            queue.return_descriptor(desc_id);
        }
    }

    pub fn queue_select(&mut self, q: u16) {
        unsafe {
            self.queue_select = q;
            write_volatile(&mut self.common.cap.queue_select, q);
        }
    }

    pub fn set_available(&mut self, desc_id: u16) {
        unsafe {
            let queue = read_volatile(self.common.cap);
            let driver_idx = (self.common.cap.queue_driver as *mut u8).offset(2) as *mut u16;
            let driver_ring_start = (self.common.cap.queue_driver as *mut u8).offset(4) as *mut u16;
            let idx = driver_idx.read_volatile();
            let elem_ptr = driver_ring_start.offset(idx as isize % queue.queue_size as isize);
            elem_ptr.write_volatile(desc_id);
            driver_idx.write_volatile(idx.wrapping_add(1));
        }
    }

    pub fn set_writable(&mut self, desc_id: u16) {
        unsafe {
            let descs = self.common.cap.queue_desc as *mut Desc;
            let mut desc = descs.offset(desc_id as isize).read_volatile();
            desc.flags = VIRTQ_DESC_F_WRITE;
            desc.len = 4096;
            descs.offset(desc_id as isize).write_volatile(desc);
        }
    }

    pub fn set_writable_available(&mut self, desc_id: u16) {
        self.set_writable(desc_id);
        self.set_available(desc_id);
    }

    pub fn add_request<T>(&mut self, desc_id: u16, desc_next_id: u16, data: T) {
        unsafe {
            let descs = self.common.cap.queue_desc as *mut Desc;
            let mut desc = descs.offset(desc_id as isize).read_volatile();
            desc.len = core::intrinsics::size_of_val(&data) as u32;
            // desc.len = data.len() as u32;
            let data_ptr = desc.addr as *mut T;
            data_ptr.write_volatile(data);

            desc.flags = VIRTQ_DESC_F_NEXT;
            desc.next = desc_next_id;
            descs.offset(desc_id as isize).write_volatile(desc);
            self.set_writable(desc_next_id);
            self.set_available(desc_id);
        };
    }

    pub fn kick(&mut self, queue_select: u16) {
        unsafe {
            let queue = read_volatile(self.common.cap);
            let VirtioCap {
                cap: cap_notify,
                bar,
                offset: offset_notify,
                length: _,
            } = &mut self.notify;

            if let Bar::Mm(addr) = bar {
                let queue_notify_address = phys_to_virt(PhysAddr::new(
                    addr.as_u64()
                        + (*offset_notify as u64)
                        + (*cap_notify as u64) * (queue.queue_notify_off as u64),
                ));

                // log::info!("kick at {:?}", queue_notify_address);
                (queue_notify_address.as_u64() as *mut u16).write_volatile(queue_select);
            }
        }
    }

    pub unsafe fn next_used(&mut self) -> Option<UsedElem> {
        let queue = read_volatile(self.common.cap);

        let device_idx = (self.common.cap.queue_device as *mut u8).offset(2) as *mut u16;
        let idx_next = device_idx.read_volatile();
        let device_ring_start =
            (self.common.cap.queue_device as *mut u8).offset(4) as *mut UsedElem;

        if let Some(virt_queue) = self.queues.get_mut(self.queue_select as usize) {
            if virt_queue.last_used_idx.wrapping_add(1) != idx_next {
                virt_queue.last_used_idx = virt_queue.last_used_idx.wrapping_add(1);
                let inq_idx = (virt_queue.last_used_idx as isize) % queue.queue_size as isize;
                let elem_ptr = device_ring_start.offset(inq_idx);
                let elem = read_volatile(elem_ptr);
                Some(elem)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn read_desc(&mut self, desc_id: u16) -> Desc {
        unsafe {
            let descs = self.common.cap.queue_desc as *mut Desc;
            descs.offset(desc_id as isize).read_volatile()
        }
    }

    /// Get device features for feature negotiation
    pub fn get_device_features(&mut self) -> u64 {
        unsafe {
            // Read lower 32 bits
            write_volatile(&mut self.common.cap.device_feature_select, 0);
            let low = read_volatile(&self.common.cap.device_feature) as u64;
            
            // Read upper 32 bits
            write_volatile(&mut self.common.cap.device_feature_select, 1);
            let high = read_volatile(&self.common.cap.device_feature) as u64;
            
            (high << 32) | low
        }
    }

    /// Set driver features for feature negotiation
    pub fn set_driver_features(&mut self, features: u64) {
        unsafe {
            // Write lower 32 bits
            write_volatile(&mut self.common.cap.driver_feature_select, 0);
            write_volatile(&mut self.common.cap.driver_feature, features as u32);
            
            // Write upper 32 bits
            write_volatile(&mut self.common.cap.driver_feature_select, 1);
            write_volatile(&mut self.common.cap.driver_feature, (features >> 32) as u32);
            
            self.driver_features = features;
        }
    }

    /// Check if a feature is supported by both device and driver
    pub fn feature_supported(&self, feature: u64) -> bool {
        (self.device_features & feature) != 0 && (self.driver_features & feature) != 0
    }

    /// Get device status
    pub fn get_device_status(&self) -> u8 {
        unsafe { read_volatile(&self.common.cap.device_status) }
    }

    /// Set device status
    pub fn set_device_status(&mut self, status: u8) {
        unsafe {
            write_volatile(&mut self.common.cap.device_status, status);
            self.status = status;
        }
    }

    /// Reset the device
    pub fn reset_device(&mut self) {
        self.set_device_status(0);
        // Wait for reset to complete
        while self.get_device_status() != 0 {
            core::hint::spin_loop();
        }
    }

    /// Read from device configuration space
    pub fn read_config_u8(&mut self, offset: u16) -> Result<u8, &'static str> {
        if offset as u32 >= self.device.length {
            return Err("Config offset out of bounds");
        }
        
        unsafe {
            let config_ptr = core::intrinsics::transmute::<&mut (), *const u8>(self.device.cap);
            Ok(read_volatile(config_ptr.offset(offset as isize)))
        }
    }

    /// Read from device configuration space (u16)
    pub fn read_config_u16(&mut self, offset: u16) -> Result<u16, &'static str> {
        if offset as u32 + 1 >= self.device.length {
            return Err("Config offset out of bounds");
        }
        
        unsafe {
            let config_ptr = core::intrinsics::transmute::<&mut (), *const u16>(self.device.cap);
            Ok(read_volatile(config_ptr.offset((offset / 2) as isize)))
        }
    }

    /// Read from device configuration space (u32)
    pub fn read_config_u32(&mut self, offset: u16) -> Result<u32, &'static str> {
        if offset as u32 + 3 >= self.device.length {
            return Err("Config offset out of bounds");
        }
        
        unsafe {
            let config_ptr = core::intrinsics::transmute::<&mut (), *const u32>(self.device.cap);
            Ok(read_volatile(config_ptr.offset((offset / 4) as isize)))
        }
    }

    /// Write to device configuration space
    pub fn write_config_u8(&mut self, offset: u16, value: u8) -> Result<(), &'static str> {
        if offset as u32 >= self.device.length {
            return Err("Config offset out of bounds");
        }
        
        unsafe {
            let config_ptr = core::intrinsics::transmute::<&mut (), *mut u8>(self.device.cap);
            write_volatile(config_ptr.offset(offset as isize), value);
        }
        Ok(())
    }

    /// Write to device configuration space (u16)
    pub fn write_config_u16(&mut self, offset: u16, value: u16) -> Result<(), &'static str> {
        if offset as u32 + 1 >= self.device.length {
            return Err("Config offset out of bounds");
        }
        
        unsafe {
            let config_ptr = core::intrinsics::transmute::<&mut (), *mut u16>(self.device.cap);
            write_volatile(config_ptr.offset((offset / 2) as isize), value);
        }
        Ok(())
    }

    /// Write to device configuration space (u32)
    pub fn write_config_u32(&mut self, offset: u16, value: u32) -> Result<(), &'static str> {
        if offset as u32 + 3 >= self.device.length {
            return Err("Config offset out of bounds");
        }
        
        unsafe {
            let config_ptr = core::intrinsics::transmute::<&mut (), *mut u32>(self.device.cap);
            write_volatile(config_ptr.offset((offset / 4) as isize), value);
        }
        Ok(())
    }

    /// Get queue information
    pub fn get_queue_info(&self, queue_idx: u16) -> Option<&VirtQueue> {
        self.queues.get(queue_idx as usize)
    }

    /// Check if indirect descriptors are supported
    pub fn supports_indirect_descriptors(&self) -> bool {
        self.feature_supported(VIRTIO_F_RING_INDIRECT_DESC)
    }

    /// Check if event index is supported
    pub fn supports_event_index(&self) -> bool {
        self.feature_supported(VIRTIO_F_RING_EVENT_IDX)
    }

    /// Enable or disable interrupts for a queue
    pub fn set_queue_interrupts(&mut self, queue_idx: u16, enabled: bool) -> Result<(), &'static str> {
        if queue_idx as usize >= self.queues.len() {
            return Err("Invalid queue index");
        }

        // This would typically modify the used ring's flags
        // Implementation depends on whether event index is supported
        if self.supports_event_index() {
            // Use event index mechanism
            // Implementation would go here
        } else {
            // Use simple interrupt suppression
            unsafe {
                let queue = &self.queues[queue_idx as usize];
                let flags_ptr = queue.driver.as_mut_ptr::<u16>();
                let current_flags = read_volatile(flags_ptr);
                if enabled {
                    write_volatile(flags_ptr, current_flags & !1); // Clear VRING_AVAIL_F_NO_INTERRUPT
                } else {
                    write_volatile(flags_ptr, current_flags | 1); // Set VRING_AVAIL_F_NO_INTERRUPT
                }
            }
        }
        
        Ok(())
    }

    /// Create a descriptor chain for complex operations
    pub fn create_descriptor_chain(&mut self, buffers: &[(u64, u32, u16)]) -> Option<u16> {
        if buffers.is_empty() {
            return None;
        }

        let mut desc_ids = Vec::new();
        
        // Allocate descriptors for the chain
        for _ in buffers {
            if let Some(desc_id) = self.get_free_desc_id() {
                desc_ids.push(desc_id);
            } else {
                // Return allocated descriptors if we can't get enough
                for id in desc_ids {
                    self.set_free_desc_id(id);
                }
                return None;
            }
        }

        // Set up the descriptor chain
        unsafe {
            let descs = self.common.cap.queue_desc as *mut Desc;
            
            for (i, ((addr, len, flags), &desc_id)) in buffers.iter().zip(desc_ids.iter()).enumerate() {
                let mut desc = Desc {
                    addr: *addr,
                    len: *len,
                    flags: *flags,
                    next: if i + 1 < desc_ids.len() { desc_ids[i + 1] } else { 0 },
                };
                
                if i + 1 < desc_ids.len() {
                    desc.flags |= VIRTQ_DESC_F_NEXT;
                }
                
                descs.offset(desc_id as isize).write_volatile(desc);
            }
        }

        desc_ids.into_iter().next()
    }

    /// Send a descriptor chain to the device and kick
    pub fn submit_chain(&mut self, head_desc_id: u16) {
        self.set_available(head_desc_id);
        self.kick(self.queue_select);
    }

    /// Check if the device has processed any requests
    pub fn has_used_descriptors(&mut self) -> bool {
        unsafe {
            let device_idx = (self.common.cap.queue_device as *mut u8).offset(2) as *mut u16;
            let idx_next = device_idx.read_volatile();
            
            if let Some(virt_queue) = self.queues.get(self.queue_select as usize) {
                virt_queue.last_used_idx.wrapping_add(1) != idx_next
            } else {
                false
            }
        }
    }

    /// Process all available used descriptors
    pub fn process_used_descriptors<F>(&mut self, mut handler: F) -> usize 
    where
        F: FnMut(UsedElem),
    {
        let mut processed = 0;
        
        while let Some(used_elem) = unsafe { self.next_used() } {
            handler(used_elem);
            processed += 1;
        }
        
        processed
    }

    /// Get current device configuration generation
    pub fn get_config_generation(&self) -> u8 {
        unsafe { read_volatile(&self.common.cap.config_generation) }
    }

    /// Check if device configuration has changed
    pub fn config_changed(&mut self) -> bool {
        let current_gen = self.get_config_generation();
        if current_gen != self.config_generation {
            self.config_generation = current_gen;
            true
        } else {
            false
        }
    }

    /// Enable MSI-X interrupts for a specific queue
    pub fn enable_msix_for_queue(&mut self, queue_idx: u16, vector: u16) -> Result<(), &'static str> {
        if queue_idx as usize >= self.queues.len() {
            return Err("Invalid queue index");
        }

        unsafe {
            // Select the queue
            write_volatile(&mut self.common.cap.queue_select, queue_idx);
            // Set the MSI-X vector
            write_volatile(&mut self.common.cap.queue_msix_vector, vector);
            
            if let Some(queue) = self.queues.get_mut(queue_idx as usize) {
                queue.msix_vector = vector;
            }
        }
        
        Ok(())
    }

    /// Disable MSI-X interrupts for a specific queue
    pub fn disable_msix_for_queue(&mut self, queue_idx: u16) -> Result<(), &'static str> {
        self.enable_msix_for_queue(queue_idx, 0xFFFF) // 0xFFFF means no vector
    }

    /// Get queue size for a specific queue
    pub fn get_queue_size(&self, queue_idx: u16) -> Option<u16> {
        self.queues.get(queue_idx as usize).map(|q| q.size)
    }

    /// Check if a queue is enabled
    pub fn is_queue_enabled(&self, queue_idx: u16) -> bool {
        self.queues.get(queue_idx as usize).map_or(false, |q| q.enabled)
    }

    /// Get total number of queues
    pub fn num_queues(&self) -> u16 {
        self.queues.len() as u16
    }

    /// Perform a complete feature negotiation sequence
    pub fn negotiate_features(&mut self, desired_features: u64) -> u64 {
        // Get device features
        self.device_features = self.get_device_features();
        
        // Select features that both device and driver support
        let negotiated = self.device_features & desired_features;
        
        // Set driver features
        self.set_driver_features(negotiated);
        
        log::debug!("VirtIO feature negotiation: device=0x{:016x}, desired=0x{:016x}, negotiated=0x{:016x}",
                   self.device_features, desired_features, negotiated);
        
        negotiated
    }
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct VirtioPciCommonCfg {
    // About the whole device.
    pub device_feature_select: u32, // read-write
    pub device_feature: u32,        // read-only for driver
    pub driver_feature_select: u32, // read-write
    pub driver_feature: u32,        // read-write
    pub msix_config: u16,           // read-write
    pub num_queues: u16,            // read-only for driver
    pub device_status: u8,          // read-write
    pub config_generation: u8,      // read-only for driver

    // About a specific virtqueue.
    pub queue_select: u16,      // read-write
    pub queue_size: u16,        // read-write
    pub queue_msix_vector: u16, // read-write
    pub queue_enable: u16,      // read-write
    pub queue_notify_off: u16,  // read-only for driver
    pub queue_desc: u64,        // read-write
    pub queue_driver: u64,      // read-write
    pub queue_device: u64,      // read-write
}

#[derive(Debug, PartialEq)]
pub struct VirtioCap<T> {
    pub cap: T,
    pub bar: Bar,
    pub offset: u32,
    pub length: u32,
}

impl<T> VirtioCap<T> {
    pub fn new(t: T, bar: Bar, offset: u32, length: u32) -> Self {
        Self {
            cap: t,
            bar,
            offset,
            length,
        }
    }
}

// Device cfg
#[repr(C)]
#[derive(Debug)]
struct VirtioInputConfig {
    select: u8,
    subsel: u8,
    size: u8,
    reserved: [u8; 5],
    u: VirtioInputUnion,
}

#[repr(C)]
#[derive(Clone, Copy)]
union VirtioInputUnion {
    string: [char; 128],
    bitmap: [u8; 128],
    abs: VirtioInputAbsInfo,
    ids: VirtioInputDevIds,
}

impl fmt::Debug for VirtioInputUnion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            f.debug_struct("VirtioInputUnion")
                .field("abs", &self.abs)
                .field("ids", &self.ids)
                .finish()
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct VirtioInputAbsInfo {
    min: u32,
    max: u32,
    fuzz: u32,
    flat: u32,
    resolution: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct VirtioInputDevIds {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

//Queue handle
#[repr(C, align(16))]
#[derive(Clone, Debug, PartialEq)]
pub struct Desc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

#[repr(C)]
#[derive(Debug)]
pub struct UsedElem {
    pub id: u32,
    pub len: u32,
}
