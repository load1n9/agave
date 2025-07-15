use crate::sys::error::{AgaveError, AgaveResult};
use alloc::alloc::GlobalAlloc;
use linked_list_allocator::LockedHeap;
use spin::Mutex;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 500 * 1024 * 1024; // 500 MiB

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Memory allocation statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub heap_size: usize,
    pub allocated: usize,
    pub peak_allocated: usize,
    pub allocation_count: u64,
    pub deallocation_count: u64,
    pub failed_allocations: u64,
}

impl MemoryStats {
    pub fn new() -> Self {
        Self {
            heap_size: HEAP_SIZE,
            allocated: 0,
            peak_allocated: 0,
            allocation_count: 0,
            deallocation_count: 0,
            failed_allocations: 0,
        }
    }

    pub fn free(&self) -> usize {
        self.heap_size.saturating_sub(self.allocated)
    }

    pub fn utilization_percent(&self) -> f32 {
        if self.heap_size == 0 {
            0.0
        } else {
            (self.allocated as f32 / self.heap_size as f32) * 100.0
        }
    }

    pub fn fragmentation_estimate(&self) -> f32 {
        // Simple fragmentation estimate based on allocation patterns
        if self.allocation_count == 0 {
            0.0
        } else {
            let avg_alloc_size = self.allocated as f32 / self.allocation_count as f32;
            // Higher ratio of small allocations suggests more fragmentation
            (1.0 - (avg_alloc_size / 1024.0).min(1.0)) * 100.0
        }
    }
}

static MEMORY_STATS: Mutex<MemoryStats> = Mutex::new(MemoryStats {
    heap_size: HEAP_SIZE,
    allocated: 0,
    peak_allocated: 0,
    allocation_count: 0,
    deallocation_count: 0,
    failed_allocations: 0,
});

/// Memory pool for specific allocation sizes
struct MemoryPool {
    block_size: usize,
    blocks: linked_list_allocator::Heap,
    allocated_blocks: usize,
    total_blocks: usize,
}

impl MemoryPool {
    pub fn new(block_size: usize, total_size: usize) -> Self {
        Self {
            block_size,
            blocks: linked_list_allocator::Heap::empty(),
            allocated_blocks: 0,
            total_blocks: total_size / block_size,
        }
    }

    pub fn allocate(&mut self) -> Option<*mut u8> {
        if self.allocated_blocks >= self.total_blocks {
            return None;
        }

        // Simplified allocation - in real implementation would use the pool
        self.allocated_blocks += 1;
        None // Placeholder
    }

    pub fn deallocate(&mut self, _ptr: *mut u8) {
        if self.allocated_blocks > 0 {
            self.allocated_blocks -= 1;
        }
    }
}

/// Enhanced allocator with tracking
pub struct AllocFromCtx;

unsafe impl core::alloc::GlobalAlloc for AllocFromCtx {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let ptr = ALLOCATOR.alloc(layout);

        // Update statistics
        if !ptr.is_null() {
            let mut stats = MEMORY_STATS.lock();
            stats.allocated += layout.size();
            stats.allocation_count += 1;
            if stats.allocated > stats.peak_allocated {
                stats.peak_allocated = stats.allocated;
            }

            // Log large allocations
            if layout.size() > 1024 * 1024 {
                log::info!("Large allocation: {} bytes", layout.size());
            }
        } else {
            let mut stats = MEMORY_STATS.lock();
            stats.failed_allocations += 1;
            log::warn!("Failed allocation: {} bytes", layout.size());
        }

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        ALLOCATOR.dealloc(ptr, layout);

        // Update statistics
        let mut stats = MEMORY_STATS.lock();
        stats.allocated = stats.allocated.saturating_sub(layout.size());
        stats.deallocation_count += 1;
    }

    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        layout: core::alloc::Layout,
        new_size: usize,
    ) -> *mut u8 {
        let new_ptr = ALLOCATOR.realloc(ptr, layout, new_size);

        // Update statistics
        if !new_ptr.is_null() {
            let mut stats = MEMORY_STATS.lock();
            stats.allocated = stats
                .allocated
                .saturating_sub(layout.size())
                .saturating_add(new_size);
            if new_size > layout.size() {
                stats.allocation_count += 1;
            }
            if stats.allocated > stats.peak_allocated {
                stats.peak_allocated = stats.allocated;
            }
        } else {
            let mut stats = MEMORY_STATS.lock();
            stats.failed_allocations += 1;
        }

        new_ptr
    }
}

pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> AgaveResult<()> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE as u64 - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(AgaveError::OutOfMemory)?;
        let flags =
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;

        match unsafe { mapper.map_to(page, frame, flags, frame_allocator) } {
            Ok(tlb) => tlb.flush(),
            Err(MapToError::FrameAllocationFailed) => return Err(AgaveError::OutOfMemory),
            Err(MapToError::ParentEntryHugePage) => return Err(AgaveError::InvalidAddress),
            Err(MapToError::PageAlreadyMapped(_)) => {
                log::warn!("Page already mapped during heap init: {:?}", page);
                continue;
            }
        }
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
    }

    log::info!(
        "Heap initialized: start=0x{:x}, size={} MB",
        HEAP_START,
        HEAP_SIZE / 1024 / 1024
    );

    Ok(())
}

/// Get current memory statistics
pub fn memory_stats() -> MemoryStats {
    MEMORY_STATS.lock().clone()
}

/// Get total heap size
pub fn memory_size() -> usize {
    HEAP_SIZE
}

/// Get currently used memory
pub fn memory_used() -> usize {
    MEMORY_STATS.lock().allocated
}

/// Get available memory
pub fn memory_free() -> usize {
    HEAP_SIZE.saturating_sub(memory_used())
}

/// Check if system is running low on memory
pub fn is_memory_low() -> bool {
    let stats = MEMORY_STATS.lock();
    stats.utilization_percent() > 85.0
}

/// Force garbage collection hint (placeholder for future GC implementation)
pub fn suggest_gc() {
    if is_memory_low() {
        log::info!(
            "Memory usage high ({}%), suggesting garbage collection",
            memory_stats().utilization_percent()
        );
    }
}

/// Memory pressure levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPressure {
    Low,      // < 50% usage
    Medium,   // 50-75% usage
    High,     // 75-90% usage
    Critical, // > 90% usage
}

/// Get current memory pressure level
pub fn memory_pressure() -> MemoryPressure {
    let utilization = memory_stats().utilization_percent();

    if utilization > 90.0 {
        MemoryPressure::Critical
    } else if utilization > 75.0 {
        MemoryPressure::High
    } else if utilization > 50.0 {
        MemoryPressure::Medium
    } else {
        MemoryPressure::Low
    }
}

/// Allocate aligned memory with error handling
pub fn allocate_aligned(size: usize, align: usize) -> AgaveResult<*mut u8> {
    use core::alloc::{GlobalAlloc, Layout};

    let layout = Layout::from_size_align(size, align).map_err(|_| AgaveError::InvalidInput)?;

    let ptr = unsafe { ALLOCATOR.alloc(layout) };
    if ptr.is_null() {
        Err(AgaveError::OutOfMemory)
    } else {
        Ok(ptr)
    }
}

/// Safely deallocate aligned memory
pub unsafe fn deallocate_aligned(ptr: *mut u8, size: usize, align: usize) -> AgaveResult<()> {
    use core::alloc::{GlobalAlloc, Layout};

    if ptr.is_null() {
        return Ok(());
    }

    let layout = Layout::from_size_align(size, align).map_err(|_| AgaveError::InvalidInput)?;

    ALLOCATOR.dealloc(ptr, layout);
    Ok(())
}
