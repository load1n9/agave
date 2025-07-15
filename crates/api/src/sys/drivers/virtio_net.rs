/// VirtIO Network device driver for Agave OS (simplified version)
use crate::sys::{
    error::{AgaveError, AgaveResult},
    network::{NetworkInterface, InterfaceState, NetworkConfig, NetworkStats},
    virtio::Virtio,
};
use alloc::{vec::Vec, string::{String, ToString}, vec};

/// Simple network queue for packet buffers
#[derive(Debug)]
pub struct NetQueue {
    buffers: Vec<Vec<u8>>,
    capacity: usize,
    bytes_processed: u64,
    packets_processed: u64,
}

impl NetQueue {
    pub fn new(capacity: usize) -> AgaveResult<Self> {
        Ok(Self {
            buffers: Vec::with_capacity(capacity),
            capacity,
            bytes_processed: 0,
            packets_processed: 0,
        })
    }

    pub fn add_buffer(&mut self, buffer: Vec<u8>) -> AgaveResult<()> {
        if self.buffers.len() < self.capacity {
            self.buffers.push(buffer);
            Ok(())
        } else {
            Err(AgaveError::InvalidInput)
        }
    }

    pub fn get_buffer(&mut self) -> Option<Vec<u8>> {
        if let Some(buffer) = self.buffers.pop() {
            self.bytes_processed += buffer.len() as u64;
            self.packets_processed += 1;
            Some(buffer)
        } else {
            None
        }
    }

    pub fn size(&self) -> usize {
        self.buffers.len()
    }

    pub fn total_bytes(&self) -> u64 {
        self.bytes_processed
    }

    pub fn packets(&self) -> u64 {
        self.packets_processed
    }
}

/// VirtIO Network device
pub struct VirtioNet {
    base: Virtio,
    rx_queue: NetQueue,
    tx_queue: NetQueue,
    mac_address: [u8; 6],
    interface_name: String,
}

impl VirtioNet {
    /// Initialize VirtIO network device
    pub fn new(virtio: Virtio) -> AgaveResult<Self> {
        let mut device = Self {
            base: virtio,
            rx_queue: NetQueue::new(256)?,
            tx_queue: NetQueue::new(256)?,
            mac_address: [0x52, 0x54, 0x00, 0x12, 0x34, 0x56], // Default QEMU MAC
            interface_name: "eth0".to_string(),
        };

        device.initialize()?;
        Ok(device)
    }

    /// Initialize the network device
    fn initialize(&mut self) -> AgaveResult<()> {
        log::info!("Initializing VirtIO network device...");

        log::info!("VirtIO Net MAC address: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                   self.mac_address[0], self.mac_address[1], self.mac_address[2],
                   self.mac_address[3], self.mac_address[4], self.mac_address[5]);

        // Set up receive buffers
        self.setup_receive_buffers()?;

        // Register network interface with network stack
        let interface = NetworkInterface {
            name: self.interface_name.clone(),
            mac_address: self.mac_address,
            config: NetworkConfig::default(),
            state: InterfaceState::Down,
            stats: NetworkStats::default(),
        };

        crate::sys::network::NETWORK_MANAGER.lock().add_interface(interface)?;
        
        log::info!("VirtIO network device initialized successfully");
        Ok(())
    }

    /// Set up receive buffers
    fn setup_receive_buffers(&mut self) -> AgaveResult<()> {
        const RX_BUFFER_SIZE: usize = 1518; // Maximum Ethernet frame size
        
        for _ in 0..256 {
            let buffer = vec![0u8; RX_BUFFER_SIZE];
            self.rx_queue.add_buffer(buffer)?;
        }

        log::debug!("Set up {} receive buffers", self.rx_queue.size());
        Ok(())
    }

    /// Send a packet
    pub fn send_packet(&mut self, packet: &[u8]) -> AgaveResult<usize> {
        if packet.len() > 1514 {
            return Err(AgaveError::InvalidInput);
        }

        // For now, just log the packet transmission
        log::trace!("Sending {} byte packet", packet.len());
        
        // TODO: Implement actual VirtIO packet transmission
        self.tx_queue.bytes_processed += packet.len() as u64;
        self.tx_queue.packets_processed += 1;

        Ok(packet.len())
    }

    /// Process received packets
    pub fn process_received_packets(&mut self) -> AgaveResult<usize> {
        let mut packets_processed = 0;

        while let Some(buffer) = self.rx_queue.get_buffer() {
            // Simulate packet processing
            log::trace!("Received packet: {:?}", buffer);

            // Parse the packet (example: Ethernet frame parsing)
            if buffer.len() >= 14 {
                let destination_mac = &buffer[0..6];
                let source_mac = &buffer[6..12];
                let ethertype = u16::from_be_bytes([buffer[12], buffer[13]]);

                log::debug!("Packet details: Destination MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}, Source MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}, Ethertype: 0x{:04x}",
                    destination_mac[0], destination_mac[1], destination_mac[2], destination_mac[3], destination_mac[4], destination_mac[5],
                    source_mac[0], source_mac[1], source_mac[2], source_mac[3], source_mac[4], source_mac[5],
                    ethertype);
            } else {
                log::warn!("Received packet is too small to parse");
            }

            // TODO: Add actual packet processing logic here

            packets_processed += 1;
        }

        log::trace!("Processed {} packets", packets_processed);
        Ok(packets_processed)
    }

    /// Get network statistics
    pub fn get_stats(&self) -> NetworkStats {
        NetworkStats {
            bytes_sent: self.tx_queue.total_bytes(),
            bytes_received: self.rx_queue.total_bytes(),
            packets_sent: self.tx_queue.packets(),
            packets_received: self.rx_queue.packets(),
            errors: 0,
            dropped: 0,
        }
    }

    /// Start the device
    pub fn start(&mut self) -> AgaveResult<()> {
        log::info!("Starting VirtIO network device");
        // TODO: Set VirtIO device status to DRIVER_OK
        Ok(())
    }

    /// Stop the device
    pub fn stop(&mut self) -> AgaveResult<()> {
        log::info!("Stopping VirtIO network device");
        // TODO: Reset VirtIO device
        Ok(())
    }

    /// Get MAC address
    pub fn mac_address(&self) -> [u8; 6] {
        self.mac_address
    }
}

/// Driver function for VirtIO network device
pub async fn drive(virtio: Virtio) {
    log::info!("Starting VirtIO network driver task");

    let mut net_device = match VirtioNet::new(virtio) {
        Ok(device) => device,
        Err(e) => {
            log::error!("Failed to initialize VirtIO network device: {:?}", e);
            return;
        }
    };

    if let Err(e) = net_device.start() {
        log::error!("Failed to start VirtIO network device: {:?}", e);
        return;
    }

    // Bring up the network interface
    {
        let mut manager = crate::sys::network::NETWORK_MANAGER.lock();
        if let Err(e) = manager.interface_up("eth0") {
            log::error!("Failed to bring up eth0 interface: {:?}", e);
        } else {
            log::info!("eth0 interface is now up");
        }
    }

    log::info!("VirtIO network driver ready");

    // Main driver loop
    loop {
        // Process any received packets
        if let Err(e) = net_device.process_received_packets() {
            log::error!("Error processing received packets: {:?}", e);
        }

        // Yield to other tasks
        crate::sys::task::executor::yield_once().await;
    }
}
