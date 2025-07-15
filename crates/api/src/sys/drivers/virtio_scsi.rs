/// VirtIO SCSI Device Driver for Agave OS
/// Provides SCSI device support through VirtIO interface

use crate::sys::{
    create_identity_virt_from_phys_n,
    error::{AgaveError, AgaveResult},
    task::executor::yield_once,
    virtio::{Desc, Virtio},
};
use alloc::{sync::Arc, vec::Vec, string::{String, ToString}, boxed::Box, vec};
use core::{
    ptr::{read_volatile, write_volatile},
    sync::atomic::{AtomicU32, Ordering},
};
use lazy_static::lazy_static;
use spin::Mutex;

/// VirtIO SCSI feature bits
const VIRTIO_SCSI_F_INOUT: u64 = 1 << 0;
const VIRTIO_SCSI_F_HOTPLUG: u64 = 1 << 1;
const VIRTIO_SCSI_F_CHANGE: u64 = 1 << 2;
const VIRTIO_SCSI_F_T10_PI: u64 = 1 << 3;

/// SCSI queue indices
const SCSI_CONTROL_QUEUE: u16 = 0;
const SCSI_EVENT_QUEUE: u16 = 1;
const SCSI_REQUEST_QUEUE: u16 = 2;

/// SCSI response codes
const VIRTIO_SCSI_S_OK: u8 = 0;
const VIRTIO_SCSI_S_OVERRUN: u8 = 1;
const VIRTIO_SCSI_S_ABORTED: u8 = 2;
const VIRTIO_SCSI_S_BAD_TARGET: u8 = 3;
const VIRTIO_SCSI_S_RESET: u8 = 4;
const VIRTIO_SCSI_S_BUSY: u8 = 5;
const VIRTIO_SCSI_S_TRANSPORT_FAILURE: u8 = 6;
const VIRTIO_SCSI_S_TARGET_FAILURE: u8 = 7;
const VIRTIO_SCSI_S_NEXUS_FAILURE: u8 = 8;
const VIRTIO_SCSI_S_FAILURE: u8 = 9;

/// SCSI task management functions
const VIRTIO_SCSI_T_TMF_ABORT_TASK: u32 = 0;
const VIRTIO_SCSI_T_TMF_ABORT_TASK_SET: u32 = 1;
const VIRTIO_SCSI_T_TMF_CLEAR_ACA: u32 = 2;
const VIRTIO_SCSI_T_TMF_CLEAR_TASK_SET: u32 = 3;
const VIRTIO_SCSI_T_TMF_I_T_NEXUS_RESET: u32 = 4;
const VIRTIO_SCSI_T_TMF_LOGICAL_UNIT_RESET: u32 = 5;
const VIRTIO_SCSI_T_TMF_QUERY_TASK: u32 = 6;
const VIRTIO_SCSI_T_TMF_QUERY_TASK_SET: u32 = 7;

/// SCSI event types
const VIRTIO_SCSI_T_EVENTS_MISSED: u32 = 0x40000000;
const VIRTIO_SCSI_T_NO_EVENT: u32 = 0;
const VIRTIO_SCSI_T_TRANSPORT_RESET: u32 = 1;
const VIRTIO_SCSI_T_ASYNC_NOTIFY: u32 = 2;
const VIRTIO_SCSI_T_PARAM_CHANGE: u32 = 3;

/// Maximum SCSI command length
const VIRTIO_SCSI_CDB_SIZE: usize = 32;
const VIRTIO_SCSI_SENSE_SIZE: usize = 96;

/// VirtIO SCSI configuration
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct VirtioScsiConfig {
    num_queues: u32,        // Number of request queues
    seg_max: u32,          // Maximum number of segments per request
    max_sectors: u32,      // Maximum number of sectors per request
    cmd_per_lun: u32,      // Maximum commands per LUN
    event_info_size: u32,  // Event information size
    sense_size: u32,       // Sense data size
    cdb_size: u32,         // Command descriptor block size
    max_channel: u16,      // Maximum channel number
    max_target: u16,       // Maximum target number
    max_lun: u32,          // Maximum LUN number
}

/// SCSI request header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct VirtioScsiReqHeader {
    lun: [u8; 8],          // Logical unit number
    tag: u64,              // Request tag
    task_attr: u8,         // Task attributes
    prio: u8,              // Priority
    crn: u8,               // Command reference number
    cdb: [u8; VIRTIO_SCSI_CDB_SIZE], // Command descriptor block
}

/// SCSI response header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct VirtioScsiRespHeader {
    sense_len: u32,        // Sense data length
    resid: u32,            // Residual data length
    status_qualifier: u16, // Status qualifier
    status: u8,            // SCSI status
    response: u8,          // VirtIO SCSI response
    sense: [u8; VIRTIO_SCSI_SENSE_SIZE], // Sense data
}

/// SCSI control request
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct VirtioScsiCtrlTmfReq {
    type_: u32,            // Request type
    subtype: u32,          // Request subtype
    lun: [u8; 8],          // Logical unit number
    tag: u64,              // Request tag
}

/// SCSI control response
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct VirtioScsiCtrlTmfResp {
    response: u8,          // Response code
}

/// SCSI event
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct VirtioScsiEvent {
    event: u32,            // Event type
    lun: [u8; 8],          // Logical unit number
    reason: u32,           // Event reason
}

/// SCSI device information
#[derive(Debug, Clone)]
pub struct ScsiDevice {
    pub channel: u16,
    pub target: u16,
    pub lun: u32,
    pub device_type: u8,
    pub vendor: String,
    pub product: String,
    pub revision: String,
    pub capacity: u64,
    pub block_size: u32,
    pub online: bool,
}

impl ScsiDevice {
    pub fn new(channel: u16, target: u16, lun: u32) -> Self {
        Self {
            channel,
            target,
            lun,
            device_type: 0,
            vendor: String::new(),
            product: String::new(),
            revision: String::new(),
            capacity: 0,
            block_size: 512,
            online: false,
        }
    }

    /// Get SCSI address as LUN array
    pub fn to_lun_array(&self) -> [u8; 8] {
        let mut lun = [0u8; 8];
        lun[0] = 1; // Address method: peripheral device addressing
        lun[1] = self.target as u8;
        lun[2] = (self.lun >> 8) as u8;
        lun[3] = (self.lun & 0xFF) as u8;
        lun
    }
}

/// SCSI command
#[derive(Debug, Clone)]
pub struct ScsiCommand {
    pub device: ScsiDevice,
    pub cdb: [u8; VIRTIO_SCSI_CDB_SIZE],
    pub cdb_len: usize,
    pub direction: ScsiDirection,
    pub data: Vec<u8>,
    pub sense_buffer: [u8; VIRTIO_SCSI_SENSE_SIZE],
    pub tag: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScsiDirection {
    None,
    ToDevice,    // Write
    FromDevice,  // Read
}

/// SCSI operation result
#[derive(Debug, Clone)]
pub struct ScsiResult {
    pub status: u8,
    pub response: u8,
    pub sense_data: Vec<u8>,
    pub residual: u32,
    pub data: Vec<u8>,
}

/// VirtIO SCSI device driver
pub struct VirtioScsi {
    virtio: Virtio,
    config: VirtioScsiConfig,
    features: u64,
    devices: Vec<ScsiDevice>,
    request_counter: AtomicU32,
    hotplug_enabled: bool,
    t10_pi_enabled: bool,
}

lazy_static! {
    static ref SCSI_DEVICE: Mutex<Option<VirtioScsi>> = Mutex::new(None);
}

impl VirtioScsi {
    /// Create new VirtIO SCSI device
    pub fn new(mut virtio: Virtio) -> AgaveResult<Self> {
        log::info!("Initializing VirtIO SCSI device");

        // Feature negotiation
        let desired_features = VIRTIO_SCSI_F_INOUT 
            | VIRTIO_SCSI_F_HOTPLUG 
            | VIRTIO_SCSI_F_CHANGE 
            | VIRTIO_SCSI_F_T10_PI;
        
        let negotiated = virtio.negotiate_features(desired_features);
        log::info!("VirtIO SCSI negotiated features: 0x{:016x}", negotiated);

        let hotplug_enabled = (negotiated & VIRTIO_SCSI_F_HOTPLUG) != 0;
        let t10_pi_enabled = (negotiated & VIRTIO_SCSI_F_T10_PI) != 0;

        // Read device configuration
        let config = Self::read_config(&mut virtio)?;
        let num_queues = unsafe { core::ptr::read_unaligned((&config as *const VirtioScsiConfig as *const u8).add(0) as *const u32) };
        let max_sectors = unsafe { core::ptr::read_unaligned((&config as *const VirtioScsiConfig as *const u8).add(8) as *const u32) };
        let cmd_per_lun = unsafe { core::ptr::read_unaligned((&config as *const VirtioScsiConfig as *const u8).add(12) as *const u32) };
        log::info!("SCSI config: {} queues, max_sectors={}, cmd_per_lun={}", num_queues, max_sectors, cmd_per_lun);
        
        let max_channel = unsafe { core::ptr::read_unaligned((&config as *const VirtioScsiConfig as *const u8).add(16) as *const u16) };
        let max_target = unsafe { core::ptr::read_unaligned((&config as *const VirtioScsiConfig as *const u8).add(18) as *const u16) };
        let max_lun = unsafe { core::ptr::read_unaligned((&config as *const VirtioScsiConfig as *const u8).add(20) as *const u32) };
        log::info!("SCSI addressing: channels=0-{}, targets=0-{}, luns=0-{}", max_channel, max_target, max_lun);

        let mut scsi = Self {
            virtio,
            config,
            features: negotiated,
            devices: Vec::new(),
            request_counter: AtomicU32::new(0),
            hotplug_enabled,
            t10_pi_enabled,
        };

        // Set up queues
        scsi.setup_queues()?;

        // Scan for devices
        scsi.scan_devices()?;

        log::info!("VirtIO SCSI device initialized with {} devices", scsi.devices.len());
        Ok(scsi)
    }

    /// Read device configuration
    fn read_config(virtio: &mut Virtio) -> AgaveResult<VirtioScsiConfig> {
        let num_queues = virtio.read_config_u32(0)?;
        let seg_max = virtio.read_config_u32(4)?;
        let max_sectors = virtio.read_config_u32(8)?;
        let cmd_per_lun = virtio.read_config_u32(12)?;
        let event_info_size = virtio.read_config_u32(16)?;
        let sense_size = virtio.read_config_u32(20)?;
        let cdb_size = virtio.read_config_u32(24)?;
        let max_channel = virtio.read_config_u16(28)?;
        let max_target = virtio.read_config_u16(30)?;
        let max_lun = virtio.read_config_u32(32)?;

        Ok(VirtioScsiConfig {
            num_queues,
            seg_max,
            max_sectors,
            cmd_per_lun,
            event_info_size,
            sense_size,
            cdb_size,
            max_channel,
            max_target,
            max_lun,
        })
    }

    /// Set up SCSI queues
    fn setup_queues(&mut self) -> AgaveResult<()> {
        // Set up event queue for hotplug events
        if self.hotplug_enabled {
            self.virtio.queue_select(SCSI_EVENT_QUEUE);
            for _ in 0..8 {
                if let Some(desc_id) = self.virtio.get_free_desc_id() {
                    self.virtio.set_writable_available(desc_id);
                }
            }
            log::debug!("SCSI event queue set up");
        }

        log::debug!("SCSI queues set up");
        Ok(())
    }

    /// Scan for SCSI devices
    fn scan_devices(&mut self) -> AgaveResult<()> {
        log::info!("Scanning for SCSI devices...");
        
        // Scan all possible addresses
        for channel in 0..=self.config.max_channel {
            for target in 0..=self.config.max_target {
                for lun in 0..=self.config.max_lun.min(255) {
                    if let Ok(device) = self.probe_device(channel, target, lun) {
                        log::info!("Found SCSI device: {}:{}:{} - {} {} {}", 
                                  channel, target, lun, 
                                  device.vendor, device.product, device.revision);
                        self.devices.push(device);
                    }
                }
            }
        }

        log::info!("SCSI device scan complete: {} devices found", self.devices.len());
        Ok(())
    }

    /// Probe for a device at specific address
    fn probe_device(&mut self, channel: u16, target: u16, lun: u32) -> AgaveResult<ScsiDevice> {
        let mut device = ScsiDevice::new(channel, target, lun);
        
        // Send INQUIRY command to probe device
        let inquiry_result = self.send_inquiry(&device)?;
        
        if inquiry_result.status == 0 && inquiry_result.data.len() >= 36 {
            let data = &inquiry_result.data;
            
            // Parse INQUIRY response
            device.device_type = data[0] & 0x1F;
            
            // Extract vendor, product, and revision
            if data.len() >= 36 {
                device.vendor = String::from_utf8_lossy(&data[8..16]).trim().to_string();
                device.product = String::from_utf8_lossy(&data[16..32]).trim().to_string();
                device.revision = String::from_utf8_lossy(&data[32..36]).trim().to_string();
            }
            
            device.online = true;
            
            // For direct access devices, get capacity
            if device.device_type == 0 {
                if let Ok(capacity_result) = self.send_read_capacity(&device) {
                    if capacity_result.data.len() >= 8 {
                        let cap_data = &capacity_result.data;
                        device.capacity = u32::from_be_bytes([cap_data[0], cap_data[1], cap_data[2], cap_data[3]]) as u64 + 1;
                        device.block_size = u32::from_be_bytes([cap_data[4], cap_data[5], cap_data[6], cap_data[7]]);
                    }
                }
            }
            
            Ok(device)
        } else {
            Err(AgaveError::NotFound)
        }
    }

    /// Send INQUIRY command
    fn send_inquiry(&mut self, device: &ScsiDevice) -> AgaveResult<ScsiResult> {
        let mut cdb = [0u8; VIRTIO_SCSI_CDB_SIZE];
        cdb[0] = 0x12; // INQUIRY
        cdb[1] = 0;    // EVPD=0, CMDDT=0
        cdb[2] = 0;    // Page code
        cdb[3] = 0;    // Reserved
        cdb[4] = 96;   // Allocation length
        cdb[5] = 0;    // Control
        
        let command = ScsiCommand {
            device: device.clone(),
            cdb,
            cdb_len: 6,
            direction: ScsiDirection::FromDevice,
            data: vec![0u8; 96],
            sense_buffer: [0u8; VIRTIO_SCSI_SENSE_SIZE],
            tag: self.request_counter.fetch_add(1, Ordering::SeqCst) as u64,
        };
        
        self.execute_command(command)
    }

    /// Send READ CAPACITY command
    fn send_read_capacity(&mut self, device: &ScsiDevice) -> AgaveResult<ScsiResult> {
        let mut cdb = [0u8; VIRTIO_SCSI_CDB_SIZE];
        cdb[0] = 0x25; // READ CAPACITY(10)
        cdb[1] = 0;    // RelAdr=0
        // LBA = 0 (bytes 2-5)
        // Reserved (bytes 6-7)
        cdb[8] = 0;    // PMI=0
        cdb[9] = 0;    // Control
        
        let command = ScsiCommand {
            device: device.clone(),
            cdb,
            cdb_len: 10,
            direction: ScsiDirection::FromDevice,
            data: vec![0u8; 8],
            sense_buffer: [0u8; VIRTIO_SCSI_SENSE_SIZE],
            tag: self.request_counter.fetch_add(1, Ordering::SeqCst) as u64,
        };
        
        self.execute_command(command)
    }

    /// Execute SCSI command
    pub fn execute_command(&mut self, mut command: ScsiCommand) -> AgaveResult<ScsiResult> {
        self.virtio.queue_select(SCSI_REQUEST_QUEUE);

        // Get descriptors for the request
        let desc_ids = self.get_descriptors_for_request(&command)?;
        
        // Set up request header
        let req_header = VirtioScsiReqHeader {
            lun: command.device.to_lun_array(),
            tag: command.tag,
            task_attr: 1, // Simple task
            prio: 0,
            crn: 0,
            cdb: command.cdb,
        };

        // Set up descriptor chain
        self.setup_scsi_descriptor_chain(&desc_ids, &req_header, &mut command)?;

        // Submit the request
        self.virtio.submit_chain(desc_ids[0]);

        // Wait for completion
        let result = self.wait_for_scsi_completion(&desc_ids)?;

        // Clean up descriptors
        for desc_id in desc_ids {
            self.virtio.set_free_desc_id(desc_id);
        }

        Ok(result)
    }

    /// Get required descriptors for a SCSI request
    fn get_descriptors_for_request(&mut self, command: &ScsiCommand) -> AgaveResult<Vec<u16>> {
        let mut desc_ids = Vec::new();

        // Request header descriptor
        if let Some(desc_id) = self.virtio.get_free_desc_id() {
            desc_ids.push(desc_id);
        } else {
            return Err(AgaveError::ResourceExhausted);
        }

        // Data descriptor (if any)
        if !command.data.is_empty() {
            if let Some(desc_id) = self.virtio.get_free_desc_id() {
                desc_ids.push(desc_id);
            } else {
                self.virtio.set_free_desc_id(desc_ids[0]);
                return Err(AgaveError::ResourceExhausted);
            }
        }

        // Response header descriptor
        if let Some(desc_id) = self.virtio.get_free_desc_id() {
            desc_ids.push(desc_id);
        } else {
            for id in &desc_ids {
                self.virtio.set_free_desc_id(*id);
            }
            return Err(AgaveError::ResourceExhausted);
        }

        Ok(desc_ids)
    }

    /// Set up descriptor chain for SCSI request
    fn setup_scsi_descriptor_chain(&mut self, _desc_ids: &[u16], header: &VirtioScsiReqHeader, 
                                  command: &mut ScsiCommand) -> AgaveResult<()> {
        let mut buffers = Vec::new();

        // Request header (read-only)
        let header_addr = header as *const _ as u64;
        buffers.push((header_addr, core::mem::size_of::<VirtioScsiReqHeader>() as u32, 0u16));

        // Data buffer (read or write depending on direction)
        if !command.data.is_empty() {
            let data_addr = command.data.as_mut_ptr() as u64;
            let flags = match command.direction {
                ScsiDirection::FromDevice => 2u16, // VIRTQ_DESC_F_WRITE
                ScsiDirection::ToDevice => 0u16,
                ScsiDirection::None => 0u16,
            };
            buffers.push((data_addr, command.data.len() as u32, flags));
        }

        // Response header (write-only)
        let resp_addr = create_identity_virt_from_phys_n(1)?.start_address().as_u64();
        buffers.push((resp_addr, core::mem::size_of::<VirtioScsiRespHeader>() as u32, 2u16));

        // Create the descriptor chain
        if self.virtio.create_descriptor_chain(&buffers).is_none() {
            return Err(AgaveError::ResourceExhausted);
        }

        Ok(())
    }

    /// Wait for SCSI command completion
    fn wait_for_scsi_completion(&mut self, desc_ids: &[u16]) -> AgaveResult<ScsiResult> {
        // Simple polling approach for now
        loop {
            if self.virtio.has_used_descriptors() {
                let mut completion_found = false;
                let mut found_desc_id = 0u16;
                
                // First pass: check if our descriptors are complete
                self.virtio.process_used_descriptors(|used_elem| {
                    if desc_ids.contains(&(used_elem.id as u16)) {
                        completion_found = true;
                        found_desc_id = used_elem.id as u16;
                    }
                });
                
                // If our descriptors completed, read the response
                if completion_found {
                    let resp_desc = self.virtio.read_desc(desc_ids.last().unwrap().clone());
                    let result = unsafe {
                        let response = read_volatile(resp_desc.addr as *const VirtioScsiRespHeader);
                        
                        let sense_data = if response.sense_len > 0 {
                            response.sense[0..response.sense_len.min(VIRTIO_SCSI_SENSE_SIZE as u32) as usize].to_vec()
                        } else {
                            Vec::new()
                        };

                        ScsiResult {
                            status: response.status,
                            response: response.response,
                            sense_data,
                            residual: response.resid,
                            data: Vec::new(), // Data is already in command.data
                        }
                    };
                    
                    return Ok(result);
                }
            }
            
            // Yield to prevent busy waiting
            core::hint::spin_loop();
        }
    }

    /// Read data from SCSI device
    pub fn read_blocks(&mut self, device_index: usize, lba: u64, block_count: u32) -> AgaveResult<Vec<u8>> {
        if device_index >= self.devices.len() {
            return Err(AgaveError::InvalidInput);
        }

        let device = &self.devices[device_index];
        if device.device_type != 0 {
            return Err(AgaveError::InvalidInput);
        }

        let data_size = block_count as usize * device.block_size as usize;
        
        let mut cdb = [0u8; VIRTIO_SCSI_CDB_SIZE];
        cdb[0] = 0x28; // READ(10)
        cdb[1] = 0;    // RelAdr=0, FUA=0, DPO=0
        cdb[2] = ((lba >> 24) & 0xFF) as u8;
        cdb[3] = ((lba >> 16) & 0xFF) as u8;
        cdb[4] = ((lba >> 8) & 0xFF) as u8;
        cdb[5] = (lba & 0xFF) as u8;
        cdb[6] = 0;    // Reserved
        cdb[7] = ((block_count >> 8) & 0xFF) as u8;
        cdb[8] = (block_count & 0xFF) as u8;
        cdb[9] = 0;    // Control

        let command = ScsiCommand {
            device: device.clone(),
            cdb,
            cdb_len: 10,
            direction: ScsiDirection::FromDevice,
            data: vec![0u8; data_size],
            sense_buffer: [0u8; VIRTIO_SCSI_SENSE_SIZE],
            tag: self.request_counter.fetch_add(1, Ordering::SeqCst) as u64,
        };

        let result = self.execute_command(command)?;
        
        if result.status == 0 && result.response == VIRTIO_SCSI_S_OK {
            Ok(result.data)
        } else {
            log::error!("SCSI read failed: status={}, response={}", result.status, result.response);
            Err(AgaveError::IoError)
        }
    }

    /// Write data to SCSI device
    pub fn write_blocks(&mut self, device_index: usize, lba: u64, data: &[u8]) -> AgaveResult<()> {
        if device_index >= self.devices.len() {
            return Err(AgaveError::InvalidInput);
        }

        let device = &self.devices[device_index];
        if device.device_type != 0 {
            return Err(AgaveError::InvalidInput);
        }

        if data.len() % device.block_size as usize != 0 {
            return Err(AgaveError::InvalidInput);
        }

        let block_count = data.len() / device.block_size as usize;
        
        let mut cdb = [0u8; VIRTIO_SCSI_CDB_SIZE];
        cdb[0] = 0x2A; // WRITE(10)
        cdb[1] = 0;    // RelAdr=0, FUA=0, DPO=0
        cdb[2] = ((lba >> 24) & 0xFF) as u8;
        cdb[3] = ((lba >> 16) & 0xFF) as u8;
        cdb[4] = ((lba >> 8) & 0xFF) as u8;
        cdb[5] = (lba & 0xFF) as u8;
        cdb[6] = 0;    // Reserved
        cdb[7] = ((block_count >> 8) & 0xFF) as u8;
        cdb[8] = (block_count & 0xFF) as u8;
        cdb[9] = 0;    // Control

        let command = ScsiCommand {
            device: device.clone(),
            cdb,
            cdb_len: 10,
            direction: ScsiDirection::ToDevice,
            data: data.to_vec(),
            sense_buffer: [0u8; VIRTIO_SCSI_SENSE_SIZE],
            tag: self.request_counter.fetch_add(1, Ordering::SeqCst) as u64,
        };

        let result = self.execute_command(command)?;
        
        if result.status == 0 && result.response == VIRTIO_SCSI_S_OK {
            Ok(())
        } else {
            log::error!("SCSI write failed: status={}, response={}", result.status, result.response);
            Err(AgaveError::IoError)
        }
    }

    /// Process SCSI events (hotplug, etc.)
    pub fn process_events(&mut self) -> AgaveResult<()> {
        if !self.hotplug_enabled {
            return Ok(());
        }

        self.virtio.queue_select(SCSI_EVENT_QUEUE);
        
        while let Some(used_elem) = unsafe { self.virtio.next_used() } {
            let desc = self.virtio.read_desc(used_elem.id as u16);
            
            unsafe {
                let event = read_volatile(desc.addr as *const VirtioScsiEvent);
                self.handle_scsi_event(&event)?;
            }
            
            // Return descriptor to available ring
            self.virtio.set_writable_available(used_elem.id as u16);
        }

        Ok(())
    }

    /// Handle SCSI event
    fn handle_scsi_event(&mut self, event: &VirtioScsiEvent) -> AgaveResult<()> {
        let event_type = unsafe { core::ptr::read_unaligned(event as *const VirtioScsiEvent as *const u32) };
        let event_reason = unsafe { core::ptr::read_unaligned((event as *const VirtioScsiEvent as *const u8).add(4) as *const u32) };
        log::debug!("SCSI event: type={}, reason={}", event_type, event_reason);

        match event_type {
            VIRTIO_SCSI_T_TRANSPORT_RESET => {
                log::info!("SCSI transport reset");
                // Re-scan devices after reset
                self.scan_devices()?;
            }
            VIRTIO_SCSI_T_PARAM_CHANGE => {
                log::info!("SCSI parameter change");
                // Update device parameters
            }
            VIRTIO_SCSI_T_ASYNC_NOTIFY => {
                log::info!("SCSI async notification");
            }
            _ => {
                log::debug!("Unknown SCSI event: {}", event_type);
            }
        }

        Ok(())
    }

    /// Get device list
    pub fn get_devices(&self) -> &Vec<ScsiDevice> {
        &self.devices
    }

    /// Get device by index
    pub fn get_device(&self, index: usize) -> Option<&ScsiDevice> {
        self.devices.get(index)
    }

    /// Get device count
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Check if hotplug is supported
    pub fn supports_hotplug(&self) -> bool {
        self.hotplug_enabled
    }

    /// Check if T10 PI is supported
    pub fn supports_t10_pi(&self) -> bool {
        self.t10_pi_enabled
    }
}

/// Public driver function
pub async fn drive(virtio: Virtio) {
    log::info!("Starting VirtIO SCSI driver");

    let scsi = match VirtioScsi::new(virtio) {
        Ok(device) => device,
        Err(e) => {
            log::error!("Failed to initialize VirtIO SCSI: {:?}", e);
            return;
        }
    };

    // Store in global instance
    *SCSI_DEVICE.lock() = Some(scsi);

    log::info!("VirtIO SCSI driver ready");

    // Main driver loop
    loop {
        // Process SCSI events
        if let Some(ref mut scsi) = SCSI_DEVICE.lock().as_mut() {
            if let Err(e) = scsi.process_events() {
                log::error!("Error processing SCSI events: {:?}", e);
            }
        }

        yield_once().await;
    }
}

/// Public API functions

/// Get SCSI device list
pub fn get_scsi_devices() -> Vec<ScsiDevice> {
    if let Some(ref scsi) = SCSI_DEVICE.lock().as_ref() {
        scsi.get_devices().clone()
    } else {
        Vec::new()
    }
}

/// Read blocks from SCSI device
pub fn read_scsi_blocks(device_index: usize, lba: u64, block_count: u32) -> AgaveResult<Vec<u8>> {
    if let Some(ref mut scsi) = SCSI_DEVICE.lock().as_mut() {
        scsi.read_blocks(device_index, lba, block_count)
    } else {
        Err(AgaveError::NotReady)
    }
}

/// Write blocks to SCSI device
pub fn write_scsi_blocks(device_index: usize, lba: u64, data: &[u8]) -> AgaveResult<()> {
    if let Some(ref mut scsi) = SCSI_DEVICE.lock().as_mut() {
        scsi.write_blocks(device_index, lba, data)
    } else {
        Err(AgaveError::NotReady)
    }
}

/// Check if SCSI is available
pub fn is_scsi_available() -> bool {
    SCSI_DEVICE.lock().is_some()
}

/// Get SCSI device count
pub fn get_scsi_device_count() -> usize {
    if let Some(ref scsi) = SCSI_DEVICE.lock().as_ref() {
        scsi.device_count()
    } else {
        0
    }
}
