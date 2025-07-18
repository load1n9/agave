use crate::sys::drivers::virtio_block::{BlockDevice, VirtioBlockDevice};
use crate::sys::error::{AgaveError, AgaveResult};
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use spin::Mutex;

pub struct VirtioBlockDisk {
    device: spin::Mutex<VirtioBlockDevice>,
}

impl VirtioBlockDisk {
    pub fn new(device: VirtioBlockDevice) -> Self {
        Self {
            device: spin::Mutex::new(device),
        }
    }
}

impl DiskBackend for VirtioBlockDisk {
    fn read_block(&self, block_num: BlockNumber, buffer: &mut Block) -> AgaveResult<()> {
        let mut dev = self.device.lock();
        let block_size = dev.block_size() as usize;
        if block_size != BLOCK_SIZE {
            return Err(AgaveError::InvalidParameter);
        }
        // Use the synchronous implementation from VirtioBlockDevice
        dev.read_block(block_num, buffer)
    }

    fn write_block(&self, block_num: BlockNumber, buffer: &Block) -> AgaveResult<()> {
        let mut dev = self.device.lock();
        let block_size = dev.block_size() as usize;
        if block_size != BLOCK_SIZE {
            return Err(AgaveError::InvalidParameter);
        }
        // Use buffer to write data to the device
        dev.write_block(block_num, buffer)
    }

    fn flush(&self) -> AgaveResult<()> {
        let mut dev = self.device.lock();
        BlockDevice::flush(&mut *dev)
    }

    fn total_blocks(&self) -> u64 {
        let dev = self.device.lock();
        BlockDevice::block_count(&*dev)
    }

    fn is_writable(&self) -> bool {
        let dev = self.device.lock();
        !dev.is_read_only()
    }

    fn backend_type(&self) -> &'static str {
        "Virtio Block Device"
    }
}

/// Block size for disk operations (4KB)
pub const BLOCK_SIZE: usize = 4096;

/// Maximum number of blocks supported
pub const MAX_BLOCKS: u64 = 1024 * 1024; // 4GB max

/// Block number type
pub type BlockNumber = u64;

/// Disk block data
pub type Block = [u8; BLOCK_SIZE];

/// Disk backend trait for different storage types
pub trait DiskBackend: Send + Sync {
    /// Read a block from disk
    fn read_block(&self, block_num: BlockNumber, buffer: &mut Block) -> AgaveResult<()>;

    /// Write a block to disk
    fn write_block(&self, block_num: BlockNumber, buffer: &Block) -> AgaveResult<()>;

    /// Flush any pending writes
    fn flush(&self) -> AgaveResult<()>;

    /// Get total number of blocks
    fn total_blocks(&self) -> u64;

    /// Check if backend supports writing
    fn is_writable(&self) -> bool;

    /// Get backend name/type
    fn backend_type(&self) -> &'static str;
}

/// Configuration for error injection in RamDisk
#[derive(Debug, Clone, Default)]
pub struct RamDiskErrorConfig {
    pub fail_reads: bool,
    pub fail_writes: bool,
    pub fail_block: Option<BlockNumber>, // Only fail for a specific block if set
}

/// RAM-based disk backend (for testing and virtual disks)
pub struct RamDisk {
    blocks: Mutex<Vec<Block>>,
    block_count: u64,
    read_only: bool,
    inject_error: Option<RamDiskErrorConfig>,
    read_count: core::sync::atomic::AtomicU64,
    write_count: core::sync::atomic::AtomicU64,
    cache: Mutex<BTreeMap<BlockNumber, Block>>, // LRU cache
}

const CACHE_SIZE: usize = 32;

impl RamDisk {
    /// Zero/wipe all blocks in the RAM disk
    pub fn wipe(&mut self) -> AgaveResult<()> {
        if self.read_only {
            return Err(AgaveError::PermissionDenied);
        }
        let mut blocks = self.blocks.lock();
        for block in blocks.iter_mut() {
            block.fill(0);
        }
        Ok(())
    }
    /// Export the entire RAM disk contents as a Vec<u8>
    pub fn export(&self) -> Vec<u8> {
        let blocks = self.blocks.lock();
        let mut data = Vec::with_capacity(self.block_count as usize * BLOCK_SIZE);
        for block in blocks.iter() {
            data.extend_from_slice(block);
        }
        data
    }

    /// Import data into the RAM disk, resizing as needed (overwrites all contents)
    pub fn import(&mut self, data: &[u8]) -> AgaveResult<()> {
        if self.read_only {
            return Err(AgaveError::PermissionDenied);
        }
        let new_block_count = (data.len() + BLOCK_SIZE - 1) / BLOCK_SIZE;
        if new_block_count == 0 || new_block_count > MAX_BLOCKS as usize {
            return Err(AgaveError::InvalidParameter);
        }
        let mut blocks = self.blocks.lock();
        blocks.clear();
        let mut offset = 0;
        for _ in 0..new_block_count {
            let mut block = [0u8; BLOCK_SIZE];
            let end = (offset + BLOCK_SIZE).min(data.len());
            if offset < data.len() {
                block[..end - offset].copy_from_slice(&data[offset..end]);
            }
            blocks.push(block);
            offset += BLOCK_SIZE;
        }
        self.block_count = new_block_count as u64;
        Ok(())
    }
    /// Resize the RAM disk to a new block count (can grow or shrink)
    pub fn resize(&mut self, new_block_count: u64) -> AgaveResult<()> {
        if self.read_only {
            return Err(AgaveError::PermissionDenied);
        }
        if new_block_count == 0 || new_block_count > MAX_BLOCKS {
            return Err(AgaveError::InvalidParameter);
        }
        let mut blocks = self.blocks.lock();
        let current = self.block_count;
        if new_block_count > current {
            let to_add = (new_block_count - current) as usize;
            blocks.reserve(to_add);
            for _ in 0..to_add {
                blocks.push([0u8; BLOCK_SIZE]);
            }
        } else if new_block_count < current {
            blocks.truncate(new_block_count as usize);
        }
        self.block_count = new_block_count;
        Ok(())
    }
    /// Create a new RAM disk with specified number of blocks
    pub fn new(block_count: u64) -> AgaveResult<Self> {
        if block_count == 0 || block_count > MAX_BLOCKS {
            return Err(AgaveError::InvalidParameter);
        }

        let mut blocks = Vec::new();
        blocks
            .try_reserve(block_count as usize)
            .map_err(|_| AgaveError::OutOfMemory)?;

        // Initialize all blocks with zeros
        for _ in 0..block_count {
            blocks.push([0u8; BLOCK_SIZE]);
        }

        log::info!(
            "Created RAM disk with {} blocks ({} MB)",
            block_count,
            (block_count * BLOCK_SIZE as u64) / (1024 * 1024)
        );

        Ok(Self {
            blocks: Mutex::new(blocks),
            block_count,
            read_only: false,
            inject_error: None,
            read_count: core::sync::atomic::AtomicU64::new(0),
            write_count: core::sync::atomic::AtomicU64::new(0),
            cache: Mutex::new(BTreeMap::new()),
        })
    }
    /// Set error injection configuration
    pub fn set_error_injection(&mut self, config: RamDiskErrorConfig) {
        self.inject_error = Some(config);
    }

    /// Create a read-only RAM disk from existing data
    pub fn from_data(data: Vec<u8>) -> AgaveResult<Self> {
        let block_count = (data.len() + BLOCK_SIZE - 1) / BLOCK_SIZE;
        let mut blocks = Vec::with_capacity(block_count);

        // Convert data into blocks
        for i in 0..block_count {
            let mut block = [0u8; BLOCK_SIZE];
            let start = i * BLOCK_SIZE;
            let end = (start + BLOCK_SIZE).min(data.len());

            if start < data.len() {
                block[..end - start].copy_from_slice(&data[start..end]);
            }

            blocks.push(block);
        }

        log::info!(
            "Created read-only RAM disk from {} bytes ({} blocks)",
            data.len(),
            block_count
        );

        Ok(Self {
            blocks: Mutex::new(blocks),
            block_count: block_count as u64,
            read_only: true,
            inject_error: None,
            read_count: core::sync::atomic::AtomicU64::new(0),
            write_count: core::sync::atomic::AtomicU64::new(0),
            cache: Mutex::new(BTreeMap::new()),
        })
    }

    /// Get disk usage statistics
    pub fn get_stats(&self) -> RamDiskStats {
        let blocks = self.blocks.lock();
        let mut used_blocks = 0;

        // Count non-zero blocks as "used"
        for block in blocks.iter() {
            if block.iter().any(|&b| b != 0) {
                used_blocks += 1;
            }
        }

        RamDiskStats {
            total_blocks: self.block_count,
            used_blocks,
            total_bytes: self.block_count * BLOCK_SIZE as u64,
            used_bytes: used_blocks * BLOCK_SIZE as u64,
            read_only: self.read_only,
        }
    }

    /// Get the number of read operations performed
    pub fn get_read_count(&self) -> u64 {
        self.read_count.load(core::sync::atomic::Ordering::Relaxed)
    }

    /// Get the number of write operations performed
    pub fn get_write_count(&self) -> u64 {
        self.write_count.load(core::sync::atomic::Ordering::Relaxed)
    }

    /// Read a block with cache support
    pub fn cached_read_block(&self, block_num: BlockNumber, buffer: &mut Block) -> AgaveResult<()> {
        if block_num >= self.block_count {
            return Err(AgaveError::InvalidParameter);
        }
        // Check cache first
        let mut cache = self.cache.lock();
        if let Some(cached) = cache.get(&block_num) {
            buffer.copy_from_slice(cached);
            return Ok(());
        }
        // Not in cache, read from disk
        let blocks = self.blocks.lock();
        buffer.copy_from_slice(&blocks[block_num as usize]);
        // Insert into cache
        cache.insert(block_num, buffer.clone());
        // Enforce cache size
        if cache.len() > CACHE_SIZE {
            let first_key = *cache.keys().next().unwrap();
            cache.remove(&first_key);
        }
        Ok(())
    }

    /// Write a block with cache support
    pub fn cached_write_block(&self, block_num: BlockNumber, buffer: &Block) -> AgaveResult<()> {
        if self.read_only {
            return Err(AgaveError::PermissionDenied);
        }
        if block_num >= self.block_count {
            return Err(AgaveError::InvalidParameter);
        }
        let mut blocks = self.blocks.lock();
        blocks[block_num as usize].copy_from_slice(buffer);
        // Update cache
        let mut cache = self.cache.lock();
        cache.insert(block_num, buffer.clone());
        if cache.len() > CACHE_SIZE {
            let first_key = *cache.keys().next().unwrap();
            cache.remove(&first_key);
        }
        Ok(())
    }
}

impl DiskBackend for RamDisk {
    fn read_block(&self, block_num: BlockNumber, buffer: &mut Block) -> AgaveResult<()> {
        if block_num >= self.block_count {
            return Err(AgaveError::InvalidParameter);
        }
        if let Some(cfg) = &self.inject_error {
            if cfg.fail_reads && (cfg.fail_block.is_none() || cfg.fail_block == Some(block_num)) {
                return Err(AgaveError::IoError);
            }
        }
        let blocks = self.blocks.lock();
        buffer.copy_from_slice(&blocks[block_num as usize]);
        self.read_count
            .fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    fn write_block(&self, block_num: BlockNumber, buffer: &Block) -> AgaveResult<()> {
        if self.read_only {
            return Err(AgaveError::PermissionDenied);
        }
        if block_num >= self.block_count {
            return Err(AgaveError::InvalidParameter);
        }
        if let Some(cfg) = &self.inject_error {
            if cfg.fail_writes && (cfg.fail_block.is_none() || cfg.fail_block == Some(block_num)) {
                return Err(AgaveError::IoError);
            }
        }
        let mut blocks = self.blocks.lock();
        blocks[block_num as usize].copy_from_slice(buffer);
        self.write_count
            .fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    fn flush(&self) -> AgaveResult<()> {
        // RAM disk doesn't need flushing
        Ok(())
    }

    fn total_blocks(&self) -> u64 {
        self.block_count
    }

    fn is_writable(&self) -> bool {
        !self.read_only
    }

    fn backend_type(&self) -> &'static str {
        if self.read_only {
            "RAM Disk (Read-Only)"
        } else {
            "RAM Disk"
        }
    }
}

/// RAM disk statistics
#[derive(Debug, Clone)]
pub struct RamDiskStats {
    pub total_blocks: u64,
    pub used_blocks: u64,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub read_only: bool,
}

/// Virtual disk backed by VirtIO block device
pub struct VirtioDisk {
    #[allow(dead_code)]
    device_id: u32,
    block_count: u64,
    read_only: bool,
}

impl VirtioDisk {
    pub fn new(device_id: u32, block_count: u64, read_only: bool) -> Self {
        log::info!(
            "Created VirtIO disk: device_id={}, blocks={}, read_only={}",
            device_id,
            block_count,
            read_only
        );

        Self {
            device_id,
            block_count,
            read_only,
        }
    }
}

impl DiskBackend for VirtioDisk {
    fn read_block(&self, block_num: BlockNumber, buffer: &mut Block) -> AgaveResult<()> {
        if block_num >= self.block_count {
            return Err(AgaveError::InvalidParameter);
        }

        // Real VirtIO block device communication
        // 1. Prepare a request descriptor for read
        // 2. Submit to VirtIO queue
        // 3. Wait for completion
        // 4. Copy data to buffer
        // NOTE: This is a stub, you must wire this to your VirtioBlockDevice
        // For demonstration, we'll assume a global device instance
        extern "Rust" {
            fn virtio_block_read(device_id: u32, block_num: u64, buffer: *mut u8, size: usize) -> i32;
        }
        let res = unsafe { virtio_block_read(self.device_id, block_num, buffer.as_mut_ptr(), buffer.len()) };
        if res != 0 {
            return Err(AgaveError::IoError);
        }
        log::trace!("VirtIO disk read: block {}", block_num);
        Ok(())
    }

    fn write_block(&self, block_num: BlockNumber, buffer: &Block) -> AgaveResult<()> {
        if self.read_only {
            return Err(AgaveError::PermissionDenied);
        }
        if block_num >= self.block_count {
            return Err(AgaveError::InvalidParameter);
        }
        // Real VirtIO block device communication
        extern "Rust" {
            fn virtio_block_write(device_id: u32, block_num: u64, buffer: *const u8, size: usize) -> i32;
        }
        let res = unsafe { virtio_block_write(self.device_id, block_num, buffer.as_ptr(), buffer.len()) };
        if res != 0 {
            return Err(AgaveError::IoError);
        }
        log::trace!("VirtIO disk write: block {}", block_num);
        Ok(())
    }

    fn flush(&self) -> AgaveResult<()> {
        // TODO: Send flush command to VirtIO device
        log::trace!("VirtIO disk flush");
        Ok(())
    }

    fn total_blocks(&self) -> u64 {
        self.block_count
    }

    fn is_writable(&self) -> bool {
        !self.read_only
    }

    fn backend_type(&self) -> &'static str {
        if self.read_only {
            "VirtIO Block Device (Read-Only)"
        } else {
            "VirtIO Block Device"
        }
    }
}

/// Disk image file backend (for file-based storage)
pub struct DiskImageFile {
    file_path: alloc::string::String,
    block_count: u64,
    read_only: bool,
}

impl DiskImageFile {
    pub fn new(file_path: alloc::string::String, block_count: u64, read_only: bool) -> Self {
        log::info!(
            "Created disk image file: path={}, blocks={}, read_only={}",
            file_path,
            block_count,
            read_only
        );
        Self {
            file_path,
            block_count,
            read_only,
        }
    }

    pub fn create_empty(file_path: alloc::string::String, block_count: u64) -> AgaveResult<Self> {
        log::info!(
            "Creating empty disk image: {} ({} blocks)",
            file_path,
            block_count
        );
        Ok(Self::new(file_path, block_count, false))
    }

    pub fn open_existing(file_path: alloc::string::String, read_only: bool) -> AgaveResult<Self> {
        let block_count = 1024; // Placeholder
        log::info!(
            "Opening existing disk image: {} (read_only={})",
            file_path,
            read_only
        );
        Ok(Self::new(file_path, block_count, read_only))
    }
}

impl DiskBackend for DiskImageFile {
    fn read_block(&self, block_num: BlockNumber, buffer: &mut Block) -> AgaveResult<()> {
        if block_num >= self.block_count {
            return Err(AgaveError::InvalidParameter);
        }
        let path_hash = self.file_path.len() as u64;
        for (i, byte) in buffer.iter_mut().enumerate() {
            *byte = ((path_hash + block_num + i as u64) & 0xFF) as u8;
        }
        log::trace!("Disk image read: {} block {}", self.file_path, block_num);
        Ok(())
    }

    // TODO: Implement actual file write logic
    fn write_block(&self, block_num: BlockNumber, _buffer: &Block) -> AgaveResult<()> {
        if self.read_only {
            return Err(AgaveError::PermissionDenied);
        }
        if block_num >= self.block_count {
            return Err(AgaveError::InvalidParameter);
        }
        log::trace!("Disk image write: {} block {}", self.file_path, block_num);
        Ok(())
    }

    fn flush(&self) -> AgaveResult<()> {
        Ok(())
    }

    fn total_blocks(&self) -> u64 {
        self.block_count
    }

    fn is_writable(&self) -> bool {
        !self.read_only
    }

    fn backend_type(&self) -> &'static str {
        if self.read_only {
            "Disk Image File (Read-Only)"
        } else {
            "Disk Image File"
        }
    }
}

/// Compound disk that combines multiple backends
pub struct CompoundDisk {
    backends: Vec<Box<dyn DiskBackend>>,
    block_offsets: Vec<u64>,
    total_blocks: u64,
}

impl CompoundDisk {
    pub fn new() -> Self {
        Self {
            backends: Vec::new(),
            block_offsets: Vec::new(),
            total_blocks: 0,
        }
    }

    /// Add a backend to the compound disk
    pub fn add_backend(&mut self, backend: Box<dyn DiskBackend>) {
        self.block_offsets.push(self.total_blocks);
        self.total_blocks += backend.total_blocks();
        self.backends.push(backend);

        log::info!(
            "Added backend to compound disk: {} blocks, total now {} blocks",
            self.backends.last().unwrap().total_blocks(),
            self.total_blocks
        );
    }

    /// Remove a backend by index (recalculates offsets and total_blocks)
    pub fn remove_backend(&mut self, index: usize) -> AgaveResult<()> {
        if index >= self.backends.len() {
            return Err(AgaveError::InvalidParameter);
        }
        self.backends.remove(index);
        self.block_offsets.remove(index);
        // Recalculate offsets and total_blocks
        self.total_blocks = 0;
        self.block_offsets.clear();
        for backend in &self.backends {
            self.block_offsets.push(self.total_blocks);
            self.total_blocks += backend.total_blocks();
        }
        Ok(())
    }

    /// Find which backend handles a given block number
    fn find_backend(&self, block_num: BlockNumber) -> AgaveResult<(usize, BlockNumber)> {
        for (i, &offset) in self.block_offsets.iter().enumerate() {
            if block_num >= offset {
                let backend_blocks = self.backends[i].total_blocks();
                if block_num < offset + backend_blocks {
                    return Ok((i, block_num - offset));
                }
            }
        }
        Err(AgaveError::InvalidParameter)
    }
}

impl DiskBackend for CompoundDisk {
    fn read_block(&self, block_num: BlockNumber, buffer: &mut Block) -> AgaveResult<()> {
        let (backend_index, local_block) = self.find_backend(block_num)?;
        self.backends[backend_index].read_block(local_block, buffer)
    }

    fn write_block(&self, block_num: BlockNumber, buffer: &Block) -> AgaveResult<()> {
        let (backend_index, local_block) = self.find_backend(block_num)?;
        self.backends[backend_index].write_block(local_block, buffer)
    }

    fn flush(&self) -> AgaveResult<()> {
        for backend in &self.backends {
            backend.flush()?;
        }
        Ok(())
    }

    fn total_blocks(&self) -> u64 {
        self.total_blocks
    }

    fn is_writable(&self) -> bool {
        self.backends.iter().any(|b| b.is_writable())
    }

    fn backend_type(&self) -> &'static str {
        "Compound Disk"
    }
}
