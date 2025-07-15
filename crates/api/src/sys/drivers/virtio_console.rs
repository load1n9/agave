/// VirtIO Console Device Driver for Agave OS
/// Provides multi-port console/serial communication through VirtIO
use crate::sys::{
    error::{AgaveError, AgaveResult},
    task::executor::yield_once,
    virtio::{Desc, Virtio},
};
use alloc::{collections::VecDeque, format, string::String, vec::Vec};
use core::ptr::read_volatile;
use futures::task::AtomicWaker;
use lazy_static::lazy_static;
use spin::Mutex;

/// VirtIO Console feature bits
const VIRTIO_CONSOLE_F_SIZE: u64 = 1 << 0;
const VIRTIO_CONSOLE_F_MULTIPORT: u64 = 1 << 1;
const VIRTIO_CONSOLE_F_EMERG_WRITE: u64 = 1 << 2;

/// Console queue indices
const CONSOLE_RX_QUEUE: u16 = 0; // receiveq port0
const CONSOLE_TX_QUEUE: u16 = 1; // transmitq port0
const CONSOLE_C_RX_QUEUE: u16 = 2; // control receiveq
const CONSOLE_C_TX_QUEUE: u16 = 3; // control transmitq

/// Console port status
const VIRTIO_CONSOLE_PORT_READY: u16 = 0;
const VIRTIO_CONSOLE_PORT_ADD: u16 = 1;
const VIRTIO_CONSOLE_PORT_REMOVE: u16 = 2;
const VIRTIO_CONSOLE_PORT_CONSOLE: u16 = 3;
const VIRTIO_CONSOLE_PORT_RESIZE: u16 = 4;
const VIRTIO_CONSOLE_PORT_OPEN: u16 = 5;
const VIRTIO_CONSOLE_PORT_NAME: u16 = 6;

/// Console device configuration
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct VirtioConsoleConfig {
    cols: u16,         // Number of columns
    rows: u16,         // Number of rows
    max_nr_ports: u32, // Maximum number of ports
    emerg_wr: u32,     // Emergency write register
}

/// Console control message
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct VirtioConsoleControl {
    id: u32,    // Port ID
    event: u16, // Event type
    value: u16, // Event value
}

/// Console port information
#[derive(Debug)]
pub struct ConsolePort {
    pub id: u32,
    pub name: String,
    pub is_console: bool,
    pub is_open: bool,
    pub rx_queue: VecDeque<u8>,
    pub tx_queue: VecDeque<u8>,
    rx_waker: Option<AtomicWaker>,
    tx_waker: Option<AtomicWaker>,
}

impl ConsolePort {
    fn new(id: u32) -> Self {
        Self {
            id,
            name: format!("port{}", id),
            is_console: id == 0, // Port 0 is typically the main console
            is_open: false,
            rx_queue: VecDeque::new(),
            tx_queue: VecDeque::new(),
            rx_waker: None,
            tx_waker: None,
        }
    }

    /// Read data from the port
    pub fn read(&mut self, buffer: &mut [u8]) -> usize {
        let mut bytes_read = 0;
        for (_i, byte) in buffer.iter_mut().enumerate() {
            if let Some(data) = self.rx_queue.pop_front() {
                *byte = data;
                bytes_read += 1;
            } else {
                break;
            }
        }
        bytes_read
    }

    /// Write data to the port
    pub fn write(&mut self, data: &[u8]) -> AgaveResult<usize> {
        for &byte in data {
            self.tx_queue.push_back(byte);
        }

        // Wake up any waiting transmission task
        if let Some(ref waker) = self.tx_waker {
            waker.wake();
        }

        Ok(data.len())
    }

    /// Check if data is available to read
    pub fn has_data(&self) -> bool {
        !self.rx_queue.is_empty()
    }

    /// Get available bytes to read
    pub fn available_bytes(&self) -> usize {
        self.rx_queue.len()
    }

    /// Check if there's space to write
    pub fn can_write(&self) -> bool {
        self.tx_queue.len() < 1024 // Arbitrary limit
    }
}

/// VirtIO Console device driver
pub struct VirtioConsole {
    virtio: Virtio,
    config: VirtioConsoleConfig,
    #[allow(dead_code)]
    features: u64,
    ports: Vec<ConsolePort>,
    multiport: bool,
    emergency_write_enabled: bool,
}

lazy_static! {
    static ref CONSOLE_WAKERS: Mutex<[Option<AtomicWaker>; 256]> =
        Mutex::new([(); 256].map(|_| None));
}

impl VirtioConsole {
    /// Create new VirtIO console device
    pub fn new(mut virtio: Virtio) -> AgaveResult<Self> {
        log::info!("Initializing VirtIO console device");

        // Feature negotiation
        let desired_features =
            VIRTIO_CONSOLE_F_SIZE | VIRTIO_CONSOLE_F_MULTIPORT | VIRTIO_CONSOLE_F_EMERG_WRITE;

        let negotiated = virtio.negotiate_features(desired_features);
        log::info!("VirtIO Console negotiated features: 0x{:016x}", negotiated);

        let multiport = (negotiated & VIRTIO_CONSOLE_F_MULTIPORT) != 0;
        let emergency_write_enabled = (negotiated & VIRTIO_CONSOLE_F_EMERG_WRITE) != 0;

        // Read device configuration
        let config = Self::read_config(&mut virtio)?;
        let cols = unsafe {
            core::ptr::read_unaligned(
                (&config as *const VirtioConsoleConfig as *const u8).add(0) as *const u16
            )
        };
        let rows = unsafe {
            core::ptr::read_unaligned(
                (&config as *const VirtioConsoleConfig as *const u8).add(2) as *const u16
            )
        };
        let max_ports = unsafe {
            core::ptr::read_unaligned(
                (&config as *const VirtioConsoleConfig as *const u8).add(4) as *const u32
            )
        };
        log::info!(
            "Console config: {}x{} chars, max {} ports",
            cols,
            rows,
            max_ports
        );

        // Initialize ports
        let mut ports = Vec::new();
        let port_count = if multiport {
            config.max_nr_ports.min(16) // Limit to reasonable number
        } else {
            1 // Single port mode
        };

        for i in 0..port_count {
            let port = ConsolePort::new(i);
            log::debug!("Created console port {}: {}", i, port.name);
            ports.push(port);
        }

        let mut console = Self {
            virtio,
            config,
            features: negotiated,
            ports,
            multiport,
            emergency_write_enabled,
        };

        // Set up initial buffers
        console.setup_receive_buffers()?;

        // If multiport, send initial control messages
        if multiport {
            console.initialize_multiport()?;
        }

        log::info!(
            "VirtIO console device initialized with {} ports",
            console.ports.len()
        );
        Ok(console)
    }

    /// Read device configuration
    fn read_config(virtio: &mut Virtio) -> AgaveResult<VirtioConsoleConfig> {
        let cols = virtio.read_config_u16(0)?;
        let rows = virtio.read_config_u16(2)?;
        let max_nr_ports = virtio.read_config_u32(4)?;
        let emerg_wr = virtio.read_config_u32(8)?;

        Ok(VirtioConsoleConfig {
            cols,
            rows,
            max_nr_ports,
            emerg_wr,
        })
    }

    /// Set up receive buffers for all ports
    fn setup_receive_buffers(&mut self) -> AgaveResult<()> {
        #[allow(dead_code)]
        const BUFFER_SIZE: usize = 1024;

        // Set up buffers for port 0 (main console)
        self.virtio.queue_select(CONSOLE_RX_QUEUE);
        for _ in 0..16 {
            if let Some(desc_id) = self.virtio.get_free_desc_id() {
                self.virtio.set_writable_available(desc_id);
            }
        }

        // If multiport, set up control receive buffers
        if self.multiport {
            self.virtio.queue_select(CONSOLE_C_RX_QUEUE);
            for _ in 0..8 {
                if let Some(desc_id) = self.virtio.get_free_desc_id() {
                    self.virtio.set_writable_available(desc_id);
                }
            }
        }

        log::debug!("Console receive buffers set up");
        Ok(())
    }

    /// Initialize multiport functionality
    fn initialize_multiport(&mut self) -> AgaveResult<()> {
        // Collect port IDs first to avoid borrowing conflict
        let port_ids: Vec<u32> = self.ports.iter().map(|port| port.id).collect();

        // Send READY message for each port
        for port_id in port_ids {
            let control_msg = VirtioConsoleControl {
                id: port_id,
                event: VIRTIO_CONSOLE_PORT_READY,
                value: 1,
            };
            self.send_control_message(&control_msg)?;
        }

        log::debug!("Multiport console initialized");
        Ok(())
    }

    /// Send control message
    fn send_control_message(&mut self, msg: &VirtioConsoleControl) -> AgaveResult<()> {
        if !self.multiport {
            return Err(AgaveError::NotImplemented);
        }

        self.virtio.queue_select(CONSOLE_C_TX_QUEUE);

        if let Some((desc_id, desc_next_id)) = self.virtio.get_free_twice_desc_id() {
            self.virtio.add_request(desc_id, desc_next_id, *msg);
            self.virtio.kick(CONSOLE_C_TX_QUEUE);

            // TODO: Wait for completion
            self.virtio.set_free_desc_id(desc_id);
            self.virtio.set_free_desc_id(desc_next_id);
        } else {
            return Err(AgaveError::ResourceExhausted);
        }

        Ok(())
    }

    /// Write data to a specific port
    pub fn write_to_port(&mut self, port_id: u32, data: &[u8]) -> AgaveResult<usize> {
        if port_id >= self.ports.len() as u32 {
            return Err(AgaveError::InvalidInput);
        }

        let port = &mut self.ports[port_id as usize];
        if !port.is_open && port.id != 0 {
            return Err(AgaveError::NotReady);
        }

        // Add data to port's transmit queue
        port.write(data)?;

        // Submit data to VirtIO queue
        self.submit_transmit_data(port_id, data)?;

        Ok(data.len())
    }

    /// Submit transmit data to VirtIO
    fn submit_transmit_data(&mut self, port_id: u32, data: &[u8]) -> AgaveResult<()> {
        let queue_id = if self.multiport && port_id > 0 {
            // For multiport, use port-specific queues
            // This is simplified - real implementation would have more queues
            CONSOLE_TX_QUEUE
        } else {
            CONSOLE_TX_QUEUE
        };

        self.virtio.queue_select(queue_id);

        if let Some((desc_id, desc_next_id)) = self.virtio.get_free_twice_desc_id() {
            // Create a copy of the data for transmission
            let data_copy = data.to_vec();

            // Set up descriptors
            unsafe {
                let descs = self.virtio.common.cap.queue_desc as *mut Desc;
                let mut desc = descs.offset(desc_id as isize).read_volatile();

                // Map data to descriptor
                desc.addr = data_copy.as_ptr() as u64;
                desc.len = data_copy.len() as u32;
                desc.flags = 1; // VIRTQ_DESC_F_NEXT
                desc.next = desc_next_id;

                descs.offset(desc_id as isize).write_volatile(desc);
            }

            self.virtio.set_writable(desc_next_id);
            self.virtio.set_available(desc_id);
            self.virtio.kick(queue_id);

            // TODO: Properly track and wait for completion
            // For now, immediately free the descriptors
            self.virtio.set_free_desc_id(desc_id);
            self.virtio.set_free_desc_id(desc_next_id);
        } else {
            return Err(AgaveError::ResourceExhausted);
        }

        Ok(())
    }

    /// Read data from a specific port
    pub fn read_from_port(&mut self, port_id: u32, buffer: &mut [u8]) -> AgaveResult<usize> {
        if port_id >= self.ports.len() as u32 {
            return Err(AgaveError::InvalidInput);
        }

        let port = &mut self.ports[port_id as usize];
        Ok(port.read(buffer))
    }

    /// Process received data
    pub fn process_received_data(&mut self) -> AgaveResult<()> {
        // Process data on main console port
        self.virtio.queue_select(CONSOLE_RX_QUEUE);

        while let Some(used_elem) = unsafe { self.virtio.next_used() } {
            let desc = self.virtio.read_desc(used_elem.id as u16);

            // Read received data
            unsafe {
                let data_ptr = desc.addr as *const u8;
                let data_len = used_elem.len as usize;

                if data_len > 0 {
                    let mut received_data = Vec::with_capacity(data_len);
                    for i in 0..data_len {
                        received_data.push(read_volatile(data_ptr.offset(i as isize)));
                    }

                    // Add to port 0's receive queue
                    if let Some(port) = self.ports.get_mut(0) {
                        for byte in received_data {
                            port.rx_queue.push_back(byte);
                        }

                        // Wake up any waiting readers
                        if let Some(ref waker) = port.rx_waker {
                            waker.wake();
                        }
                    }
                }
            }

            // Return descriptor to available ring
            self.virtio.set_writable_available(used_elem.id as u16);
        }

        // Process control messages if multiport
        if self.multiport {
            self.process_control_messages()?;
        }

        Ok(())
    }

    /// Process control messages for multiport
    fn process_control_messages(&mut self) -> AgaveResult<()> {
        self.virtio.queue_select(CONSOLE_C_RX_QUEUE);

        while let Some(used_elem) = unsafe { self.virtio.next_used() } {
            let desc = self.virtio.read_desc(used_elem.id as u16);

            unsafe {
                let control_msg = read_volatile(desc.addr as *const VirtioConsoleControl);
                self.handle_control_message(&control_msg)?;
            }

            // Return descriptor
            self.virtio.set_writable_available(used_elem.id as u16);
        }

        Ok(())
    }

    /// Handle incoming control message
    fn handle_control_message(&mut self, msg: &VirtioConsoleControl) -> AgaveResult<()> {
        let msg_id =
            unsafe { core::ptr::read_unaligned(msg as *const VirtioConsoleControl as *const u16) };
        let msg_event = unsafe {
            core::ptr::read_unaligned(
                (msg as *const VirtioConsoleControl as *const u8).add(2) as *const u16
            )
        };
        let msg_value = unsafe {
            core::ptr::read_unaligned(
                (msg as *const VirtioConsoleControl as *const u8).add(4) as *const u16
            )
        };

        log::debug!(
            "Console control message: port {}, event {}, value {}",
            msg_id,
            msg_event,
            msg_value
        );

        match msg_event {
            VIRTIO_CONSOLE_PORT_ADD => {
                log::info!("Console port {} added", msg_id);
                // Port should already exist from initialization
            }
            VIRTIO_CONSOLE_PORT_REMOVE => {
                log::info!("Console port {} removed", msg_id);
                if let Some(port) = self.ports.get_mut(msg_id as usize) {
                    port.is_open = false;
                }
            }
            VIRTIO_CONSOLE_PORT_OPEN => {
                log::info!("Console port {} opened", msg_id);
                if let Some(port) = self.ports.get_mut(msg_id as usize) {
                    port.is_open = true;
                }
            }
            VIRTIO_CONSOLE_PORT_CONSOLE => {
                log::info!("Console port {} marked as console", msg_id);
                if let Some(port) = self.ports.get_mut(msg_id as usize) {
                    port.is_console = true;
                }
            }
            VIRTIO_CONSOLE_PORT_RESIZE => {
                log::info!("Console port {} resized", msg_id);
                // Update configuration if needed
            }
            VIRTIO_CONSOLE_PORT_NAME => {
                log::info!("Console port {} name event", msg_id);
                // Port name would be in separate data buffer
            }
            _ => {
                log::warn!("Unknown console control event: {}", msg_event);
            }
        }

        Ok(())
    }

    /// Write to main console (port 0)
    pub fn write(&mut self, data: &[u8]) -> AgaveResult<usize> {
        self.write_to_port(0, data)
    }

    /// Read from main console (port 0)
    pub fn read(&mut self, buffer: &mut [u8]) -> AgaveResult<usize> {
        self.read_from_port(0, buffer)
    }

    /// Emergency write (if supported)
    pub fn emergency_write(&mut self, byte: u8) -> AgaveResult<()> {
        if !self.emergency_write_enabled {
            return Err(AgaveError::NotImplemented);
        }

        // Write to emergency register
        self.virtio.write_config_u32(8, byte as u32)?;
        Ok(())
    }

    /// Get console dimensions
    pub fn dimensions(&self) -> (u16, u16) {
        (self.config.cols, self.config.rows)
    }

    /// Get number of ports
    pub fn port_count(&self) -> usize {
        self.ports.len()
    }

    /// Check if a port is open
    pub fn is_port_open(&self, port_id: u32) -> bool {
        self.ports
            .get(port_id as usize)
            .map(|port| port.is_open || port.id == 0) // Port 0 is always considered open
            .unwrap_or(false)
    }

    /// Get port information
    pub fn get_port_info(&self, port_id: u32) -> Option<&ConsolePort> {
        self.ports.get(port_id as usize)
    }

    /// Check if multiport is enabled
    pub fn is_multiport(&self) -> bool {
        self.multiport
    }
}

lazy_static! {
    pub static ref VIRTIO_CONSOLE: Mutex<Option<VirtioConsole>> = Mutex::new(None);
}

/// Console writer for formatted output
pub struct ConsoleWriter;

impl core::fmt::Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if let Some(ref mut console) = VIRTIO_CONSOLE.lock().as_mut() {
            console.write(s.as_bytes()).map_err(|_| core::fmt::Error)?;
        }
        Ok(())
    }
}

/// Print to VirtIO console
#[macro_export]
macro_rules! console_print {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            let mut writer = $crate::sys::drivers::virtio_console::ConsoleWriter;
            write!(writer, $($arg)*).unwrap();
        }
    };
}

/// Print line to VirtIO console
#[macro_export]
macro_rules! console_println {
    () => (console_print!("\n"));
    ($($arg:tt)*) => (console_print!("{}\n", format_args!($($arg)*)));
}

/// Public driver function
pub async fn drive(virtio: Virtio) {
    log::info!("Starting VirtIO console driver");

    let console = match VirtioConsole::new(virtio) {
        Ok(device) => device,
        Err(e) => {
            log::error!("Failed to initialize VirtIO console: {:?}", e);
            return;
        }
    };

    // Store in global instance
    *VIRTIO_CONSOLE.lock() = Some(console);

    log::info!("VirtIO console driver ready");

    // Main driver loop
    loop {
        // Process received data
        if let Some(ref mut console) = VIRTIO_CONSOLE.lock().as_mut() {
            if let Err(e) = console.process_received_data() {
                log::error!("Error processing console data: {:?}", e);
            }
        }

        yield_once().await;
    }
}

/// High-level console API
pub fn write_console(data: &[u8]) -> AgaveResult<usize> {
    if let Some(ref mut console) = VIRTIO_CONSOLE.lock().as_mut() {
        console.write(data)
    } else {
        Err(AgaveError::NotReady)
    }
}

pub fn read_console(buffer: &mut [u8]) -> AgaveResult<usize> {
    if let Some(ref mut console) = VIRTIO_CONSOLE.lock().as_mut() {
        console.read(buffer)
    } else {
        Err(AgaveError::NotReady)
    }
}

pub fn write_to_port(port_id: u32, data: &[u8]) -> AgaveResult<usize> {
    if let Some(ref mut console) = VIRTIO_CONSOLE.lock().as_mut() {
        console.write_to_port(port_id, data)
    } else {
        Err(AgaveError::NotReady)
    }
}

pub fn read_from_port(port_id: u32, buffer: &mut [u8]) -> AgaveResult<usize> {
    if let Some(ref mut console) = VIRTIO_CONSOLE.lock().as_mut() {
        console.read_from_port(port_id, buffer)
    } else {
        Err(AgaveError::NotReady)
    }
}

pub fn get_console_dimensions() -> Option<(u16, u16)> {
    VIRTIO_CONSOLE
        .lock()
        .as_ref()
        .map(|console| console.dimensions())
}

pub fn is_console_ready() -> bool {
    VIRTIO_CONSOLE.lock().is_some()
}
