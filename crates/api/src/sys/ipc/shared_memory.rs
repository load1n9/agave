/// Shared memory implementation for IPC
use crate::sys::{
    error::{AgaveError, AgaveResult},
    ipc::{IpcPermissions, ProcessId},
    // memory::with_mapper_framealloc, // TODO: implement memory mapping
};
use alloc::{sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

/// Shared memory segment
#[derive(Debug, Clone)]
pub struct SharedMemorySegment {
    data: Arc<Mutex<Vec<u8>>>,
    size: usize,
    owner: ProcessId,
    permissions: IpcPermissions,
    attached_processes: Arc<AtomicUsize>,
    creation_time: u64,
    last_accessed: u64,
}

impl SharedMemorySegment {
    /// Create a new shared memory segment
    pub fn new(size: usize, owner: ProcessId, permissions: IpcPermissions) -> AgaveResult<Self> {
        if size == 0 || size > MAX_SHARED_MEMORY_SIZE {
            return Err(AgaveError::InvalidParameter);
        }

        let current_time = crate::sys::interrupts::TIME_MS.load(Ordering::Relaxed);

        // Allocate the memory buffer
        let mut data = Vec::new();
        data.try_reserve(size)
            .map_err(|_| AgaveError::OutOfMemory)?;
        data.resize(size, 0);

        Ok(Self {
            data: Arc::new(Mutex::new(data)),
            size,
            owner,
            permissions,
            attached_processes: Arc::new(AtomicUsize::new(1)), // Owner is attached
            creation_time: current_time,
            last_accessed: current_time,
        })
    }

    /// Get the size of the shared memory segment
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get the owner process ID
    pub fn owner(&self) -> ProcessId {
        self.owner
    }

    /// Get permissions
    pub fn permissions(&self) -> IpcPermissions {
        self.permissions
    }

    /// Check if a process can read from this segment
    pub fn can_read(&self, process: ProcessId) -> bool {
        if process == self.owner {
            self.permissions.owner_read
        } else {
            // For simplicity, treat all non-owner processes as "other"
            // In a real system, you'd check group membership
            self.permissions.other_read
        }
    }

    /// Check if a process can write to this segment
    pub fn can_write(&self, process: ProcessId) -> bool {
        if process == self.owner {
            self.permissions.owner_write
        } else {
            self.permissions.other_write
        }
    }

    /// Attach a process to this shared memory segment
    pub fn attach(&self) -> AgaveResult<()> {
        let current_count = self.attached_processes.load(Ordering::Relaxed);
        if current_count >= MAX_ATTACHED_PROCESSES {
            return Err(AgaveError::ResourceExhausted);
        }

        self.attached_processes.fetch_add(1, Ordering::Relaxed);
        log::debug!(
            "Process attached to shared memory segment (total: {})",
            current_count + 1
        );
        Ok(())
    }

    /// Detach a process from this shared memory segment
    pub fn detach(&self) -> AgaveResult<()> {
        let current_count = self.attached_processes.load(Ordering::Relaxed);
        if current_count == 0 {
            return Err(AgaveError::InvalidOperation);
        }

        self.attached_processes.fetch_sub(1, Ordering::Relaxed);
        log::debug!(
            "Process detached from shared memory segment (remaining: {})",
            current_count - 1
        );
        Ok(())
    }

    /// Get number of attached processes
    pub fn attached_count(&self) -> usize {
        self.attached_processes.load(Ordering::Relaxed)
    }

    /// Read data from the shared memory segment
    pub fn read(&self, offset: usize, buffer: &mut [u8]) -> AgaveResult<usize> {
        if offset >= self.size {
            return Ok(0); // EOF
        }

        let data = self.data.lock();
        let bytes_to_read = buffer.len().min(self.size - offset);

        buffer[..bytes_to_read].copy_from_slice(&data[offset..offset + bytes_to_read]);

        self.update_access_time();
        log::trace!(
            "Shared memory read: {} bytes at offset {}",
            bytes_to_read,
            offset
        );
        Ok(bytes_to_read)
    }

    /// Write data to the shared memory segment
    pub fn write(&mut self, offset: usize, data: &[u8]) -> AgaveResult<()> {
        if offset >= self.size {
            return Err(AgaveError::InvalidParameter);
        }

        let bytes_to_write = data.len().min(self.size - offset);
        if bytes_to_write == 0 {
            return Ok(());
        }

        let mut buffer = self.data.lock();
        buffer[offset..offset + bytes_to_write].copy_from_slice(&data[..bytes_to_write]);

        self.update_access_time();
        log::trace!(
            "Shared memory write: {} bytes at offset {}",
            bytes_to_write,
            offset
        );
        Ok(())
    }

    /// Zero out the entire shared memory segment
    pub fn zero(&mut self) -> AgaveResult<()> {
        let mut data = self.data.lock();
        data.fill(0);
        self.update_access_time();
        log::debug!("Shared memory segment zeroed ({} bytes)", self.size);
        Ok(())
    }

    /// Get a direct reference to the underlying data (unsafe)
    /// This is for advanced use cases where zero-copy access is needed
    pub unsafe fn get_raw_ptr(&self) -> *mut u8 {
        let data = self.data.lock();
        data.as_ptr() as *mut u8
    }

    /// Update last access time
    fn update_access_time(&self) {
        // Note: We can't modify self.last_accessed directly due to borrowing rules
        // In a real implementation, you'd use an atomic or separate tracking
    }

    /// Get statistics for this shared memory segment
    pub fn get_stats(&self) -> SharedMemoryStats {
        SharedMemoryStats {
            size: self.size,
            owner: self.owner,
            attached_processes: self.attached_count(),
            creation_time: self.creation_time,
            last_accessed: self.last_accessed,
            permissions: self.permissions,
        }
    }
}

/// Shared memory statistics
#[derive(Debug, Clone)]
pub struct SharedMemoryStats {
    pub size: usize,
    pub owner: ProcessId,
    pub attached_processes: usize,
    pub creation_time: u64,
    pub last_accessed: u64,
    pub permissions: IpcPermissions,
}

/// Constants for shared memory limits
pub const MAX_SHARED_MEMORY_SIZE: usize = 16 * 1024 * 1024; // 16MB per segment
pub const MAX_ATTACHED_PROCESSES: usize = 256; // Maximum processes that can attach

/// Shared memory key type for System V style shared memory
pub type ShmKey = i32;

/// System V style shared memory segment
#[derive(Debug)]
pub struct SysVSharedMemory {
    pub key: ShmKey,
    pub segment: SharedMemorySegment,
    pub flags: ShmFlags,
}

/// Shared memory flags (System V style)
#[derive(Debug, Clone, Copy)]
pub struct ShmFlags {
    pub create: bool,
    pub exclusive: bool,
    pub read_only: bool,
}

impl Default for ShmFlags {
    fn default() -> Self {
        Self {
            create: false,
            exclusive: false,
            read_only: false,
        }
    }
}

/// Memory mapping information for shared memory
#[derive(Debug)]
pub struct MemoryMapping {
    pub virtual_address: usize,
    pub physical_address: usize,
    pub size: usize,
    pub flags: MappingFlags,
}

/// Memory mapping flags
#[derive(Debug, Clone, Copy)]
pub struct MappingFlags {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub shared: bool,
}

impl Default for MappingFlags {
    fn default() -> Self {
        Self {
            readable: true,
            writable: true,
            executable: false,
            shared: true,
        }
    }
}

/// Map shared memory into a process's address space
pub fn map_shared_memory(
    segment: &SharedMemorySegment,
    _process: ProcessId,
    flags: MappingFlags,
) -> AgaveResult<MemoryMapping> {
    // In a real implementation, this would:
    // 1. Allocate virtual address space
    // 2. Map physical pages to virtual addresses
    // 3. Set up page table entries with appropriate permissions
    // 4. Return the mapping information

    // For now, we'll simulate this
    let virtual_address = 0x10000000; // Simulated virtual address
    let physical_address = 0x20000000; // Simulated physical address

    log::debug!(
        "Mapped shared memory segment: virt=0x{:x}, phys=0x{:x}, size={}",
        virtual_address,
        physical_address,
        segment.size()
    );

    Ok(MemoryMapping {
        virtual_address,
        physical_address,
        size: segment.size(),
        flags,
    })
}

/// Unmap shared memory from a process's address space
pub fn unmap_shared_memory(mapping: &MemoryMapping) -> AgaveResult<()> {
    // In a real implementation, this would:
    // 1. Remove page table entries
    // 2. Invalidate TLB entries
    // 3. Free virtual address space

    log::debug!(
        "Unmapped shared memory: virt=0x{:x}, size={}",
        mapping.virtual_address,
        mapping.size
    );

    Ok(())
}

/// Allocate physically contiguous memory for shared memory
/// This is useful for DMA operations or when sharing with hardware
pub fn allocate_contiguous_shared_memory(
    size: usize,
    _alignment: usize,
) -> AgaveResult<SharedMemorySegment> {
    if size == 0 || size > MAX_SHARED_MEMORY_SIZE {
        return Err(AgaveError::InvalidParameter);
    }

    // TODO: In a real implementation, this would use the physical memory allocator
    // to get contiguous physical pages and map them to virtual memory
    // For now, we simulate this with a simple allocation
    SharedMemorySegment::new(size, 0, IpcPermissions::default())
}
