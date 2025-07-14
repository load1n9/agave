/// VirtIO network driver implementation
use crate::sys::error::{AgaveError, AgaveResult};
use crate::sys::network;
use alloc::{vec::Vec, string::String};
use core::ptr;

/// VirtIO network device register offsets
const VIRTIO_NET_F_CSUM: u32 = 0;
const VIRTIO_NET_F_GUEST_CSUM: u32 = 1;
const VIRTIO_NET_F_MAC: u32 = 5;
const VIRTIO_NET_F_STATUS: u32 = 16;

/// VirtIO network device configuration
#[repr(C)]
pub struct VirtioNetConfig {
    pub mac: [u8; 6],
    pub status: u16,
    pub max_virtqueue_pairs: u16,
    pub mtu: u16,
}

/// VirtIO network device
pub struct VirtioNetDevice {
    base_addr: usize,
    config: VirtioNetConfig,
    rx_queue: VirtQueue,
    tx_queue: VirtQueue,
    status: NetworkDeviceStatus,
}

/// Network device status
#[derive(Debug, Clone, PartialEq)]
pub enum NetworkDeviceStatus {
    Inactive,
    Initializing,
    Active,
    Error,
}

/// VirtIO queue (simplified)
pub struct VirtQueue {
    queue_size: u16,
    buffers: Vec<VirtQueueBuffer>,
    available_idx: u16,
    used_idx: u16,
}

/// VirtIO queue buffer
pub struct VirtQueueBuffer {
    addr: usize,
    len: u32,
    flags: u16,
    next: u16,
}

impl VirtioNetDevice {
    /// Create new VirtIO network device
    pub fn new(base_addr: usize) -> AgaveResult<Self> {
        let mut device = Self {
            base_addr,
            config: VirtioNetConfig {
                mac: [0; 6],
                status: 0,
                max_virtqueue_pairs: 1,
                mtu: 1500,
            },
            rx_queue: VirtQueue::new(256)?,
            tx_queue: VirtQueue::new(256)?,
            status: NetworkDeviceStatus::Inactive,
        };

        device.initialize()?;
        Ok(device)
    }

    /// Initialize the VirtIO network device
    fn initialize(&mut self) -> AgaveResult<()> {
        log::info!("Initializing VirtIO network device at 0x{:x}", self.base_addr);
        self.status = NetworkDeviceStatus::Initializing;

        // Reset device
        self.write_device_status(0);

        // Acknowledge device
        self.write_device_status(1);

        // Driver acknowledgment
        self.write_device_status(1 | 2);

        // Read features
        let features = self.read_device_features();
        log::info!("VirtIO net features: 0x{:x}", features);

        // Check for MAC address feature
        if (features & (1 << VIRTIO_NET_F_MAC)) != 0 {
            self.read_mac_address();
            log::info!("VirtIO net MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                       self.config.mac[0], self.config.mac[1], self.config.mac[2],
                       self.config.mac[3], self.config.mac[4], self.config.mac[5]);
        } else {
            // Use default MAC
            self.config.mac = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
            log::warn!("Using default MAC address");
        }

        // Setup virtqueues
        self.setup_queues()?;

        // Driver ready
        self.write_device_status(1 | 2 | 4);

        self.status = NetworkDeviceStatus::Active;
        log::info!("VirtIO network device initialized successfully");
        
        Ok(())
    }

    /// Setup virtqueues for TX and RX
    fn setup_queues(&mut self) -> AgaveResult<()> {
        // Setup RX queue (queue 0)
        self.setup_queue(0, &mut self.rx_queue)?;
        
        // Setup TX queue (queue 1) 
        self.setup_queue(1, &mut self.tx_queue)?;

        // Fill RX queue with buffers
        self.fill_rx_queue()?;

        Ok(())
    }

    /// Setup individual virtqueue
    fn setup_queue(&self, queue_index: u16, queue: &mut VirtQueue) -> AgaveResult<()> {
        // Select queue
        self.write_queue_select(queue_index);

        // Check queue size
        let max_size = self.read_queue_size();
        if max_size == 0 {
            return Err(AgaveError::InvalidInput);
        }

        // Use smaller of requested and max size
        let queue_size = queue.queue_size.min(max_size);
        self.write_queue_size(queue_size);

        log::debug!("Setup queue {} with size {}", queue_index, queue_size);
        Ok(())
    }

    /// Fill RX queue with receive buffers
    fn fill_rx_queue(&mut self) -> AgaveResult<()> {
        for i in 0..self.rx_queue.queue_size {
            // Allocate receive buffer (1526 bytes for jumbo frames)
            let buffer_size = 1526;
            let buffer = vec![0u8; buffer_size];
            let buffer_addr = buffer.as_ptr() as usize;
            
            // Add to available ring
            self.rx_queue.buffers[i as usize] = VirtQueueBuffer {
                addr: buffer_addr,
                len: buffer_size as u32,
                flags: 2, // VIRTQ_DESC_F_WRITE
                next: 0,
            };

            // Leak the buffer so it stays allocated
            core::mem::forget(buffer);
        }

        // Notify device of available buffers
        self.notify_queue(0);
        
        log::debug!("Filled RX queue with {} buffers", self.rx_queue.queue_size);
        Ok(())
    }

    /// Send packet
    pub fn send_packet(&mut self, packet: &[u8]) -> AgaveResult<usize> {
        if self.status != NetworkDeviceStatus::Active {
            return Err(AgaveError::NotReady);
        }

        if packet.len() > 1514 {
            return Err(AgaveError::InvalidInput);
        }

        // Get next available TX buffer
        let desc_idx = self.tx_queue.available_idx % self.tx_queue.queue_size;
        let buffer = &mut self.tx_queue.buffers[desc_idx as usize];

        // Copy packet to buffer
        unsafe {
            ptr::copy_nonoverlapping(packet.as_ptr(), buffer.addr as *mut u8, packet.len());
        }
        
        buffer.len = packet.len() as u32;
        buffer.flags = 0; // No flags for transmit

        // Update available ring
        self.tx_queue.available_idx = self.tx_queue.available_idx.wrapping_add(1);

        // Notify device
        self.notify_queue(1);

        log::trace!("Sent {} byte packet", packet.len());
        Ok(packet.len())
    }

    /// Process received packets
    pub fn process_received_packets(&mut self) -> AgaveResult<usize> {
        let mut packets_processed = 0;

        // Check for received packets
        while self.has_received_packets() {
            if let Some(packet) = self.get_received_packet()? {
                // Process packet through network stack
                network::receive_packet("eth0", &packet)?;
                packets_processed += 1;
            }
        }

        Ok(packets_processed)
    }

    /// Check if there are received packets
    fn has_received_packets(&self) -> bool {
        self.rx_queue.used_idx != self.read_used_idx(0)
    }

    /// Get received packet
    fn get_received_packet(&mut self) -> AgaveResult<Option<Vec<u8>>> {
        if !self.has_received_packets() {
            return Ok(None);
        }

        let used_idx = self.rx_queue.used_idx % self.rx_queue.queue_size;
        let buffer = &self.rx_queue.buffers[used_idx as usize];

        // Read packet data
        let packet_len = buffer.len as usize;
        let mut packet = vec![0u8; packet_len];
        
        unsafe {
            ptr::copy_nonoverlapping(buffer.addr as *const u8, packet.as_mut_ptr(), packet_len);
        }

        // Update used index
        self.rx_queue.used_idx = self.rx_queue.used_idx.wrapping_add(1);

        // Refill RX buffer
        self.refill_rx_buffer(used_idx)?;

        Ok(Some(packet))
    }

    /// Refill RX buffer after use
    fn refill_rx_buffer(&mut self, buffer_idx: u16) -> AgaveResult<()> {
        let buffer = &mut self.rx_queue.buffers[buffer_idx as usize];
        
        // Reset buffer for reuse
        buffer.len = 1526;
        buffer.flags = 2; // VIRTQ_DESC_F_WRITE

        // Notify device of available buffer
        self.notify_queue(0);

        Ok(())
    }

    /// Read MAC address from device config
    fn read_mac_address(&mut self) {
        for i in 0..6 {
            self.config.mac[i] = self.read_config_byte(i);
        }
    }

    /// Device register access methods
    fn read_device_features(&self) -> u32 {
        unsafe { ptr::read_volatile((self.base_addr + 0x10) as *const u32) }
    }

    fn write_device_status(&self, status: u8) {
        unsafe { ptr::write_volatile((self.base_addr + 0x70) as *mut u8, status); }
    }

    fn write_queue_select(&self, queue: u16) {
        unsafe { ptr::write_volatile((self.base_addr + 0x30) as *mut u16, queue); }
    }

    fn read_queue_size(&self) -> u16 {
        unsafe { ptr::read_volatile((self.base_addr + 0x38) as *const u16) }
    }

    fn write_queue_size(&self, size: u16) {
        unsafe { ptr::write_volatile((self.base_addr + 0x38) as *mut u16, size); }
    }

    fn notify_queue(&self, queue: u16) {
        unsafe { ptr::write_volatile((self.base_addr + 0x50) as *mut u16, queue); }
    }

    fn read_used_idx(&self, _queue: u16) -> u16 {
        // Simplified - in real implementation would read from queue structure
        0
    }

    fn read_config_byte(&self, offset: usize) -> u8 {
        unsafe { ptr::read_volatile((self.base_addr + 0x100 + offset) as *const u8) }
    }

    /// Get device MAC address
    pub fn mac_address(&self) -> [u8; 6] {
        self.config.mac
    }

    /// Get device status
    pub fn device_status(&self) -> NetworkDeviceStatus {
        self.status.clone()
    }
}

impl VirtQueue {
    fn new(size: u16) -> AgaveResult<Self> {
        Ok(Self {
            queue_size: size,
            buffers: vec![VirtQueueBuffer {
                addr: 0,
                len: 0,
                flags: 0,
                next: 0,
            }; size as usize],
            available_idx: 0,
            used_idx: 0,
        })
    }
}

/// Global VirtIO network device instance
static mut VIRTIO_NET_DEVICE: Option<VirtioNetDevice> = None;

/// Initialize VirtIO network device
pub fn init_virtio_net(base_addr: usize) -> AgaveResult<()> {
    log::info!("Initializing VirtIO network driver...");

    let device = VirtioNetDevice::new(base_addr)?;
    
    unsafe {
        VIRTIO_NET_DEVICE = Some(device);
    }

    // Register interface with network stack
    let interface = network::NetworkInterface {
        name: "eth0".to_string(),
        mac_address: unsafe { VIRTIO_NET_DEVICE.as_ref().unwrap().mac_address() },
        config: network::NetworkConfig::default(),
        state: network::InterfaceState::Down,
        stats: network::NetworkStats::default(),
    };

    // This will be called from kernel initialization
    log::info!("VirtIO network driver initialized");
    Ok(())
}

/// Send packet through VirtIO device
pub fn virtio_send_packet(packet: &[u8]) -> AgaveResult<usize> {
    unsafe {
        if let Some(device) = &mut VIRTIO_NET_DEVICE {
            device.send_packet(packet)
        } else {
            Err(AgaveError::NotFound)
        }
    }
}

/// Process received packets
pub fn virtio_process_packets() -> AgaveResult<usize> {
    unsafe {
        if let Some(device) = &mut VIRTIO_NET_DEVICE {
            device.process_received_packets()
        } else {
            Ok(0)
        }
    }
}

/// Check if VirtIO network device is available
pub fn is_virtio_net_available() -> bool {
    unsafe { VIRTIO_NET_DEVICE.is_some() }
}
