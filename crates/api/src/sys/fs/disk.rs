/// Disk backend for persistent storage
use crate::sys::error::{AgaveError, AgaveResult};
use alloc::{boxed::Box, vec::Vec};
use spin::Mutex;

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

/// RAM-based disk backend (for testing and virtual disks)
pub struct RamDisk {
    blocks: Mutex<Vec<Block>>,
    block_count: u64,
    read_only: bool,
}

impl RamDisk {
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
        })
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
}

impl DiskBackend for RamDisk {
    fn read_block(&self, block_num: BlockNumber, buffer: &mut Block) -> AgaveResult<()> {
        if block_num >= self.block_count {
            return Err(AgaveError::InvalidParameter);
        }

        let blocks = self.blocks.lock();
        buffer.copy_from_slice(&blocks[block_num as usize]);
        Ok(())
    }

    fn write_block(&self, block_num: BlockNumber, buffer: &Block) -> AgaveResult<()> {
        if self.read_only {
            return Err(AgaveError::PermissionDenied);
        }

        if block_num >= self.block_count {
            return Err(AgaveError::InvalidParameter);
        }

        let mut blocks = self.blocks.lock();
        blocks[block_num as usize].copy_from_slice(buffer);
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

        // TODO: Implement actual VirtIO block device communication
        // For now, simulate by filling with pattern
        for (i, byte) in buffer.iter_mut().enumerate() {
            *byte = ((block_num + i as u64) & 0xFF) as u8;
        }

        log::trace!("VirtIO disk read: block {}", block_num);
        Ok(())
    }

    fn write_block(&self, block_num: BlockNumber, _buffer: &Block) -> AgaveResult<()> {
        if self.read_only {
            return Err(AgaveError::PermissionDenied);
        }

        if block_num >= self.block_count {
            return Err(AgaveError::InvalidParameter);
        }

        // TODO: Implement actual VirtIO block device communication
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
    // In a real implementation, this would hold a file handle
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
        // TODO: Create actual file with specified size
        log::info!(
            "Creating empty disk image: {} ({} blocks)",
            file_path,
            block_count
        );
        Ok(Self::new(file_path, block_count, false))
    }

    pub fn open_existing(file_path: alloc::string::String, read_only: bool) -> AgaveResult<Self> {
        // TODO: Open existing file and determine size
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

        // TODO: Implement actual file I/O
        // For now, fill with a pattern based on the file path hash and block number
        let path_hash = self.file_path.len() as u64;
        for (i, byte) in buffer.iter_mut().enumerate() {
            *byte = ((path_hash + block_num + i as u64) & 0xFF) as u8;
        }

        log::trace!("Disk image read: {} block {}", self.file_path, block_num);
        Ok(())
    }

    fn write_block(&self, block_num: BlockNumber, _buffer: &Block) -> AgaveResult<()> {
        if self.read_only {
            return Err(AgaveError::PermissionDenied);
        }

        if block_num >= self.block_count {
            return Err(AgaveError::InvalidParameter);
        }

        // TODO: Implement actual file I/O
        log::trace!("Disk image write: {} block {}", self.file_path, block_num);
        Ok(())
    }

    fn flush(&self) -> AgaveResult<()> {
        if !self.read_only {
            // TODO: Flush file buffers to disk
            log::trace!("Disk image flush: {}", self.file_path);
        }
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
