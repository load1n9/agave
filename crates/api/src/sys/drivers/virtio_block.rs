/// VirtIO Block Device Driver for Agave OS
/// Provides storage device support through VirtIO block interface
use crate::sys::{
    create_identity_virt_from_phys_n,
    error::{AgaveError, AgaveResult},
    task::executor::yield_once,
    virtio::Virtio,
};
use alloc::{vec, vec::Vec};
use core::{ptr::read_volatile, sync::atomic::AtomicU64};
use futures::task::AtomicWaker;
use lazy_static::lazy_static;
use spin::Mutex;

/// VirtIO Block device feature bits
const VIRTIO_BLK_F_SIZE_MAX: u64 = 1 << 1;
const VIRTIO_BLK_F_SEG_MAX: u64 = 1 << 2;
const VIRTIO_BLK_F_GEOMETRY: u64 = 1 << 4;
const VIRTIO_BLK_F_RO: u64 = 1 << 5;
const VIRTIO_BLK_F_BLK_SIZE: u64 = 1 << 6;
const VIRTIO_BLK_F_TOPOLOGY: u64 = 1 << 10;
#[allow(dead_code)]
const VIRTIO_BLK_F_MQ: u64 = 1 << 12;
const VIRTIO_BLK_F_DISCARD: u64 = 1 << 13;
const VIRTIO_BLK_F_WRITE_ZEROES: u64 = 1 << 14;

/// VirtIO Block device commands
const VIRTIO_BLK_T_IN: u32 = 0;
const VIRTIO_BLK_T_OUT: u32 = 1;
const VIRTIO_BLK_T_FLUSH: u32 = 4;
const VIRTIO_BLK_T_DISCARD: u32 = 11;
const VIRTIO_BLK_T_WRITE_ZEROES: u32 = 13;

/// VirtIO Block device status
const VIRTIO_BLK_S_OK: u8 = 0;
const VIRTIO_BLK_S_IOERR: u8 = 1;
const VIRTIO_BLK_S_UNSUPP: u8 = 2;

/// Maximum sector size
const SECTOR_SIZE: usize = 512;
const MAX_SECTORS_PER_REQUEST: u32 = 256;

/// Block device configuration
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtioBlkConfig {
    pub capacity: u64, // Device capacity in 512-byte sectors
    pub size_max: u32, // Maximum segment size
    pub seg_max: u32,  // Maximum number of segments
    pub geometry: VirtioBlkGeometry,
    pub blk_size: u32, // Block size in bytes
    pub topology: VirtioBlkTopology,
    pub writeback: u8, // Writeback mode
    pub unused0: [u8; 3],
    pub max_discard_sectors: u32,      // Maximum discard sectors
    pub max_discard_seg: u32,          // Maximum discard segments
    pub discard_sector_alignment: u32, // Discard sector alignment
    pub max_write_zeroes_sectors: u32, // Maximum write zeroes sectors
    pub max_write_zeroes_seg: u32,     // Maximum write zeroes segments
    pub write_zeroes_may_unmap: u8,    // Write zeroes may unmap
    pub unused1: [u8; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtioBlkGeometry {
    pub cylinders: u16,
    pub heads: u8,
    pub sectors: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtioBlkTopology {
    pub physical_block_exp: u8, // Physical block size = 2^(physical_block_exp + 9)
    pub alignment_offset: u8,   // Alignment offset
    pub min_io_size: u16,       // Minimum I/O size
    pub opt_io_size: u32,       // Optimal I/O size
}

/// VirtIO Block request header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtioBlkReqHeader {
    pub type_: u32, // Request type (read/write/flush)
    pub reserved: u32,
    pub sector: u64, // Starting sector number
}

/// Block I/O request
#[derive(Debug)]
pub struct BlockRequest {
    pub request_id: u64,
    pub operation: BlockOperation,
    pub sector: u64,
    pub data: Vec<u8>,
    pub status: BlockRequestStatus,
    #[allow(dead_code)]
    waker: Option<AtomicWaker>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockOperation {
    Read,
    Write,
    Flush,
    Discard,
    WriteZeroes,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockRequestStatus {
    Pending,
    InProgress,
    Completed,
    Failed(u8),
}

/// Block device statistics
#[derive(Debug, Default, Clone)]
pub struct BlockStats {
    pub reads: u64,
    pub writes: u64,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub errors: u64,
    pub flush_ops: u64,
    pub discard_ops: u64,
}

/// VirtIO Block device driver
pub struct VirtioBlockDevice {
    virtio: Virtio,
    config: VirtioBlkConfig,
    features: u64,
    stats: BlockStats,
    #[allow(dead_code)]
    request_counter: AtomicU64,
    #[allow(dead_code)]
    pending_requests: Vec<BlockRequest>,
    read_only: bool,
}

lazy_static! {
    static ref BLOCK_WAKERS: Mutex<[Option<AtomicWaker>; 256]> =
        Mutex::new([(); 256].map(|_| None));
}

impl VirtioBlockDevice {
    /// Create new VirtIO block device
    pub fn new(mut virtio: Virtio) -> AgaveResult<Self> {
        log::info!("Initializing VirtIO block device");

        // Feature negotiation
        let desired_features = VIRTIO_BLK_F_SIZE_MAX
            | VIRTIO_BLK_F_SEG_MAX
            | VIRTIO_BLK_F_GEOMETRY
            | VIRTIO_BLK_F_BLK_SIZE
            | VIRTIO_BLK_F_TOPOLOGY
            | VIRTIO_BLK_F_DISCARD
            | VIRTIO_BLK_F_WRITE_ZEROES;

        let negotiated = virtio.negotiate_features(desired_features);
        log::info!("VirtIO Block negotiated features: 0x{:016x}", negotiated);

        let read_only = (negotiated & VIRTIO_BLK_F_RO) != 0;
        if read_only {
            log::info!("Block device is read-only");
        }

        // Read device configuration
        let config = Self::read_config(&mut virtio)?;
        log::info!(
            "Block device capacity: {} sectors ({} bytes)",
            config.capacity,
            config.capacity * SECTOR_SIZE as u64
        );
        log::info!("Block size: {} bytes", config.blk_size);

        Ok(Self {
            virtio,
            config,
            features: negotiated,
            stats: BlockStats::default(),
            request_counter: AtomicU64::new(0),
            pending_requests: Vec::new(),
            read_only,
        })
    }

    /// Read device configuration from VirtIO config space
    fn read_config(virtio: &mut Virtio) -> AgaveResult<VirtioBlkConfig> {
        // Read configuration fields
        let capacity =
            virtio.read_config_u32(0)? as u64 | ((virtio.read_config_u32(4)? as u64) << 32);
        let size_max = virtio.read_config_u32(8)?;
        let seg_max = virtio.read_config_u32(12)?;

        // Read geometry
        let cylinders = virtio.read_config_u16(16)?;
        let heads = virtio.read_config_u8(18)?;
        let sectors = virtio.read_config_u8(19)?;

        let blk_size = virtio.read_config_u32(20)?;

        // Read topology
        let physical_block_exp = virtio.read_config_u8(24)?;
        let alignment_offset = virtio.read_config_u8(25)?;
        let min_io_size = virtio.read_config_u16(26)?;
        let opt_io_size = virtio.read_config_u32(28)?;

        Ok(VirtioBlkConfig {
            capacity,
            size_max,
            seg_max,
            geometry: VirtioBlkGeometry {
                cylinders,
                heads,
                sectors,
            },
            blk_size,
            topology: VirtioBlkTopology {
                physical_block_exp,
                alignment_offset,
                min_io_size,
                opt_io_size,
            },
            writeback: virtio.read_config_u8(32)?,
            unused0: [0; 3],
            max_discard_sectors: virtio.read_config_u32(36).unwrap_or(0),
            max_discard_seg: virtio.read_config_u32(40).unwrap_or(0),
            discard_sector_alignment: virtio.read_config_u32(44).unwrap_or(0),
            max_write_zeroes_sectors: virtio.read_config_u32(48).unwrap_or(0),
            max_write_zeroes_seg: virtio.read_config_u32(52).unwrap_or(0),
            write_zeroes_may_unmap: virtio.read_config_u8(56).unwrap_or(0),
            unused1: [0; 3],
        })
    }

    /// Read sectors from the block device
    pub async fn read_sectors(
        &mut self,
        start_sector: u64,
        sector_count: u32,
    ) -> AgaveResult<Vec<u8>> {
        if sector_count == 0 || sector_count > MAX_SECTORS_PER_REQUEST {
            return Err(AgaveError::InvalidInput);
        }

        let data_size = sector_count as usize * SECTOR_SIZE;
        let mut buffer = vec![0u8; data_size];

        self.submit_request(BlockOperation::Read, start_sector, &mut buffer)
            .await?;

        self.stats.reads += 1;
        self.stats.bytes_read += data_size as u64;

        Ok(buffer)
    }

    /// Write sectors to the block device
    pub async fn write_sectors(&mut self, start_sector: u64, data: &[u8]) -> AgaveResult<()> {
        if self.read_only {
            return Err(AgaveError::PermissionDenied);
        }

        if data.len() % SECTOR_SIZE != 0 {
            return Err(AgaveError::InvalidInput);
        }

        let sector_count = data.len() / SECTOR_SIZE;
        if sector_count > MAX_SECTORS_PER_REQUEST as usize {
            return Err(AgaveError::InvalidInput);
        }

        let mut buffer = data.to_vec();
        self.submit_request(BlockOperation::Write, start_sector, &mut buffer)
            .await?;

        self.stats.writes += 1;
        self.stats.bytes_written += data.len() as u64;

        Ok(())
    }

    /// Flush the device write cache
    pub async fn flush(&mut self) -> AgaveResult<()> {
        let mut empty_buffer = Vec::new();
        self.submit_request(BlockOperation::Flush, 0, &mut empty_buffer)
            .await?;
        self.stats.flush_ops += 1;
        Ok(())
    }

    /// Discard sectors (TRIM command)
    pub async fn discard_sectors(
        &mut self,
        start_sector: u64,
        sector_count: u32,
    ) -> AgaveResult<()> {
        if self.read_only || (self.features & VIRTIO_BLK_F_DISCARD) == 0 {
            return Err(AgaveError::NotImplemented);
        }

        if sector_count == 0 || sector_count > self.config.max_discard_sectors {
            return Err(AgaveError::InvalidInput);
        }

        let mut empty_buffer = Vec::new();
        self.submit_request(BlockOperation::Discard, start_sector, &mut empty_buffer)
            .await?;
        self.stats.discard_ops += 1;
        Ok(())
    }

    /// Submit a block I/O request
    async fn submit_request(
        &mut self,
        operation: BlockOperation,
        sector: u64,
        buffer: &mut [u8],
    ) -> AgaveResult<()> {
        // Select the first queue (assuming single queue for simplicity)
        self.virtio.queue_select(0);

        // Get descriptors for the request
        let desc_ids = self.get_descriptors_for_request(operation, buffer.len())?;

        // Set up the request
        let request_header = VirtioBlkReqHeader {
            type_: match operation {
                BlockOperation::Read => VIRTIO_BLK_T_IN,
                BlockOperation::Write => VIRTIO_BLK_T_OUT,
                BlockOperation::Flush => VIRTIO_BLK_T_FLUSH,
                BlockOperation::Discard => VIRTIO_BLK_T_DISCARD,
                BlockOperation::WriteZeroes => VIRTIO_BLK_T_WRITE_ZEROES,
            },
            reserved: 0,
            sector,
        };

        // Map buffers and set up descriptor chain
        self.setup_descriptor_chain(&desc_ids, &request_header, buffer, operation)?;

        // Submit the request
        self.virtio.submit_chain(desc_ids[0]);

        // Wait for completion
        self.wait_for_completion(desc_ids[0]).await?;

        // Clean up descriptors
        for desc_id in desc_ids {
            self.virtio.set_free_desc_id(desc_id);
        }

        Ok(())
    }

    /// Get required descriptors for a request
    fn get_descriptors_for_request(
        &mut self,
        operation: BlockOperation,
        data_size: usize,
    ) -> AgaveResult<Vec<u16>> {
        // We need: header descriptor + data descriptor(s) + status descriptor
        let mut desc_ids = Vec::new();

        // Header descriptor
        if let Some(desc_id) = self.virtio.get_free_desc_id() {
            desc_ids.push(desc_id);
        } else {
            return Err(AgaveError::ResourceExhausted);
        }

        // Data descriptor(s) - only for read/write operations
        if matches!(operation, BlockOperation::Read | BlockOperation::Write) && data_size > 0 {
            let segments_needed = (data_size + 4095) / 4096; // Round up to page size
            for _ in 0..segments_needed {
                if let Some(desc_id) = self.virtio.get_free_desc_id() {
                    desc_ids.push(desc_id);
                } else {
                    // Return allocated descriptors if we can't get enough
                    for id in &desc_ids {
                        self.virtio.set_free_desc_id(*id);
                    }
                    return Err(AgaveError::ResourceExhausted);
                }
            }
        }

        // Status descriptor
        if let Some(desc_id) = self.virtio.get_free_desc_id() {
            desc_ids.push(desc_id);
        } else {
            // Return allocated descriptors if we can't get enough
            for id in &desc_ids {
                self.virtio.set_free_desc_id(*id);
            }
            return Err(AgaveError::ResourceExhausted);
        }

        Ok(desc_ids)
    }

    /// Set up the descriptor chain for a block request
    fn setup_descriptor_chain(
        &mut self,
        _desc_ids: &[u16],
        header: &VirtioBlkReqHeader,
        buffer: &mut [u8],
        operation: BlockOperation,
    ) -> AgaveResult<()> {
        // Create descriptor chain based on operation type
        let mut buffers = Vec::new();

        // Header buffer (always first)
        let header_addr = header as *const _ as u64;
        buffers.push((
            header_addr,
            core::mem::size_of::<VirtioBlkReqHeader>() as u32,
            0u16,
        ));

        // Data buffer(s) for read/write operations
        if matches!(operation, BlockOperation::Read | BlockOperation::Write) && !buffer.is_empty() {
            let data_addr = buffer.as_ptr() as u64;
            let flags = if matches!(operation, BlockOperation::Read) {
                2u16
            } else {
                0u16
            }; // VIRTQ_DESC_F_WRITE for reads
            buffers.push((data_addr, buffer.len() as u32, flags));
        }

        // Status buffer (always last, write-only)
        let status_addr = create_identity_virt_from_phys_n(1)?
            .start_address()
            .as_u64();
        buffers.push((status_addr, 1, 2u16)); // VIRTQ_DESC_F_WRITE

        // Create the descriptor chain
        if let Some(_head_desc) = self.virtio.create_descriptor_chain(&buffers) {
            Ok(())
        } else {
            Err(AgaveError::ResourceExhausted)
        }
    }

    /// Wait for request completion
    async fn wait_for_completion(&mut self, desc_id: u16) -> AgaveResult<()> {
        // Use a simple polling approach for now
        // In a real implementation, this would use interrupts
        loop {
            if self.virtio.has_used_descriptors() {
                let mut completion_found = false;
                #[allow(unused_assignments)]
                let mut block_status = VIRTIO_BLK_S_OK;

                // First pass: check if our descriptor is complete
                let _processed = self.virtio.process_used_descriptors(|used_elem| {
                    if used_elem.id == desc_id as u32 {
                        completion_found = true;
                    }
                });

                // If our descriptor completed, read its status and handle completion
                if completion_found {
                    let status_desc = self.virtio.read_desc(desc_id);
                    unsafe {
                        block_status = read_volatile(
                            (status_desc.addr + status_desc.len as u64 - 1) as *const u8,
                        );
                    }

                    match block_status {
                        VIRTIO_BLK_S_OK => {
                            log::trace!("Block request completed successfully");
                        }
                        VIRTIO_BLK_S_IOERR => {
                            log::error!("Block I/O error");
                            return Err(AgaveError::IoError);
                        }
                        VIRTIO_BLK_S_UNSUPP => {
                            log::error!("Block operation not supported");
                            return Err(AgaveError::NotImplemented);
                        }
                        _ => {
                            log::error!("Unknown block status: {}", block_status);
                            return Err(AgaveError::IoError);
                        }
                    }
                    break; // Request completed
                }
            }
            yield_once().await;
        }
        Ok(())
    }

    /// Get device capacity in sectors
    pub fn capacity_sectors(&self) -> u64 {
        self.config.capacity
    }

    /// Get device capacity in bytes
    pub fn capacity_bytes(&self) -> u64 {
        self.config.capacity * SECTOR_SIZE as u64
    }

    /// Get device block size
    pub fn block_size(&self) -> u32 {
        self.config.blk_size
    }

    /// Check if device is read-only
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// Get device statistics
    pub fn stats(&self) -> &BlockStats {
        &self.stats
    }

    /// Get device geometry
    pub fn geometry(&self) -> VirtioBlkGeometry {
        self.config.geometry
    }

    /// Get optimal I/O size
    pub fn optimal_io_size(&self) -> u32 {
        self.config.topology.opt_io_size
    }

    /// Check if discard is supported
    pub fn supports_discard(&self) -> bool {
        (self.features & VIRTIO_BLK_F_DISCARD) != 0
    }

    /// Check if write zeroes is supported
    pub fn supports_write_zeroes(&self) -> bool {
        (self.features & VIRTIO_BLK_F_WRITE_ZEROES) != 0
    }
}

/// Public driver function
pub async fn drive(virtio: Virtio) {
    log::info!("Starting VirtIO block device driver");

    let block_device = match VirtioBlockDevice::new(virtio) {
        Ok(device) => device,
        Err(e) => {
            log::error!("Failed to initialize VirtIO block device: {:?}", e);
            return;
        }
    };

    log::info!(
        "VirtIO block device initialized: {} sectors, block size {} bytes",
        block_device.capacity_sectors(),
        block_device.block_size()
    );

    // TODO: Register the block device with the filesystem layer
    // For now, just keep the driver running
    loop {
        // Process any pending I/O requests
        // In a real implementation, this would handle requests from the filesystem layer
        yield_once().await;
    }
}

/// High-level block device interface for filesystem layer
pub trait BlockDevice {
    fn read_block(&mut self, block_num: u64, buffer: &mut [u8]) -> AgaveResult<()>;
    fn write_block(&mut self, block_num: u64, buffer: &[u8]) -> AgaveResult<()>;
    fn flush(&mut self) -> AgaveResult<()>;
    fn block_size(&self) -> u32;
    fn block_count(&self) -> u64;
}

// Implement BlockDevice trait for VirtioBlockDevice
impl BlockDevice for VirtioBlockDevice {
    fn read_block(&mut self, block_num: u64, buffer: &mut [u8]) -> AgaveResult<()> {
        if buffer.len() != self.block_size() as usize {
            return Err(AgaveError::InvalidInput);
        }

        // Convert block number to sector number
        let sectors_per_block = self.block_size() / SECTOR_SIZE as u32;
        let _start_sector = block_num * sectors_per_block as u64;

        // This would need to be made async in a real implementation
        // For now, return an error indicating async operation needed
        Err(AgaveError::NotImplemented)
    }

    fn write_block(&mut self, block_num: u64, buffer: &[u8]) -> AgaveResult<()> {
        if self.read_only {
            return Err(AgaveError::PermissionDenied);
        }

        if buffer.len() != self.block_size() as usize {
            return Err(AgaveError::InvalidInput);
        }

        // Convert block number to sector number
        let sectors_per_block = self.block_size() / SECTOR_SIZE as u32;
        let _start_sector = block_num * sectors_per_block as u64;

        // This would need to be made async in a real implementation
        // For now, return an error indicating async operation needed
        Err(AgaveError::NotImplemented)
    }

    fn flush(&mut self) -> AgaveResult<()> {
        // This would need to be made async in a real implementation
        // For now, return an error indicating async operation needed
        Err(AgaveError::NotImplemented)
    }

    fn block_size(&self) -> u32 {
        self.config.blk_size
    }

    fn block_count(&self) -> u64 {
        self.config.capacity * SECTOR_SIZE as u64 / self.config.blk_size as u64
    }
}
