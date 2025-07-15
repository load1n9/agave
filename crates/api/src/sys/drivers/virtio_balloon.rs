/// VirtIO Memory Balloon Device Driver for Agave OS
/// Provides dynamic memory management between guest and host
use crate::sys::{
    create_identity_virt_from_phys_n,
    error::{AgaveError, AgaveResult},
    memory::BootInfoFrameAllocator,
    task::executor::yield_once,
    virtio::{Desc, Virtio},
    FRAME_ALLOCATOR, MAPPER,
};
use alloc::{sync::Arc, vec::Vec};
use core::{
    ptr::{read_volatile, write_volatile},
    sync::atomic::{AtomicU32, AtomicU64, Ordering},
};
use lazy_static::lazy_static;
use spin::Mutex;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, Size4KiB},
    PhysAddr,
};

/// VirtIO Balloon feature bits
const VIRTIO_BALLOON_F_MUST_TELL_HOST: u64 = 1 << 0;
const VIRTIO_BALLOON_F_STATS_VQ: u64 = 1 << 1;
const VIRTIO_BALLOON_F_DEFLATE_ON_OOM: u64 = 1 << 2;
const VIRTIO_BALLOON_F_FREE_PAGE_HINT: u64 = 1 << 3;
const VIRTIO_BALLOON_F_PAGE_POISON: u64 = 1 << 4;
const VIRTIO_BALLOON_F_PAGE_REPORTING: u64 = 1 << 5;

/// Balloon queue indices
const BALLOON_INFLATE_QUEUE: u16 = 0;
const BALLOON_DEFLATE_QUEUE: u16 = 1;
const BALLOON_STATS_QUEUE: u16 = 2;
const BALLOON_FREE_PAGE_HINT_QUEUE: u16 = 3;
const BALLOON_REPORTING_QUEUE: u16 = 4;

/// Page size for balloon operations (4KB)
const BALLOON_PAGE_SIZE: usize = 4096;

/// Maximum pages to process in one operation
const MAX_PAGES_PER_OPERATION: usize = 256;

/// Balloon device configuration
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct VirtioBalloonConfig {
    num_pages: u32,             // Number of pages to balloon
    actual: u32,                // Actual number of pages ballooned
    free_page_hint_cmd_id: u32, // Free page hint command ID
    poison_val: u32,            // Page poison value
}

/// Balloon statistics tags
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum BalloonStatTag {
    SwapIn = 0,
    SwapOut = 1,
    MajorFaults = 2,
    MinorFaults = 3,
    FreeMemory = 4,
    TotalMemory = 5,
    AvailableMemory = 6,
    DiskCaches = 7,
    HugetlbAllocations = 8,
    HugetlbFailures = 9,
}

/// Balloon statistics entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct VirtioBalloonStat {
    tag: u16, // BalloonStatTag
    val: u64, // Statistics value
}

/// Memory balloon statistics
#[derive(Debug, Clone, Default)]
pub struct BalloonStats {
    pub pages_inflated: u64,
    pub pages_deflated: u64,
    pub current_pages: u32,
    pub target_pages: u32,
    pub free_memory: u64,
    pub total_memory: u64,
    pub available_memory: u64,
    pub major_faults: u64,
    pub minor_faults: u64,
}

/// Page frame for balloon operations
#[derive(Debug, Clone)]
struct BalloonPage {
    frame: PhysFrame<Size4KiB>,
    address: PhysAddr,
}

/// VirtIO Memory Balloon device driver
pub struct VirtioBalloon {
    virtio: Virtio,
    config: VirtioBalloonConfig,
    features: u64,
    stats: BalloonStats,
    inflated_pages: Vec<BalloonPage>,
    stats_enabled: bool,
    deflate_on_oom: bool,
    page_reporting: bool,
    last_stats_update: AtomicU64,
}

lazy_static! {
    static ref BALLOON_DEVICE: Mutex<Option<VirtioBalloon>> = Mutex::new(None);
}

impl VirtioBalloon {
    /// Create new VirtIO balloon device
    pub fn new(mut virtio: Virtio) -> AgaveResult<Self> {
        log::info!("Initializing VirtIO memory balloon device");

        // Feature negotiation
        let desired_features = VIRTIO_BALLOON_F_MUST_TELL_HOST
            | VIRTIO_BALLOON_F_STATS_VQ
            | VIRTIO_BALLOON_F_DEFLATE_ON_OOM
            | VIRTIO_BALLOON_F_PAGE_REPORTING;

        let negotiated = virtio.negotiate_features(desired_features);
        log::info!("VirtIO Balloon negotiated features: 0x{:016x}", negotiated);

        let stats_enabled = (negotiated & VIRTIO_BALLOON_F_STATS_VQ) != 0;
        let deflate_on_oom = (negotiated & VIRTIO_BALLOON_F_DEFLATE_ON_OOM) != 0;
        let page_reporting = (negotiated & VIRTIO_BALLOON_F_PAGE_REPORTING) != 0;

        // Read device configuration
        let config = Self::read_config(&mut virtio)?;
        let num_pages = unsafe {
            core::ptr::read_unaligned(
                (&config as *const VirtioBalloonConfig as *const u8).add(0) as *const u32
            )
        };
        let actual = unsafe {
            core::ptr::read_unaligned(
                (&config as *const VirtioBalloonConfig as *const u8).add(4) as *const u32
            )
        };
        log::info!(
            "Balloon config: target={} pages, actual={} pages",
            num_pages,
            actual
        );

        let mut balloon = Self {
            virtio,
            config,
            features: negotiated,
            stats: BalloonStats {
                target_pages: config.num_pages,
                current_pages: config.actual,
                ..Default::default()
            },
            inflated_pages: Vec::new(),
            stats_enabled,
            deflate_on_oom,
            page_reporting,
            last_stats_update: AtomicU64::new(0),
        };

        // Set up initial buffers
        balloon.setup_queues()?;

        // Send initial statistics if enabled
        if stats_enabled {
            balloon.send_statistics()?;
        }

        log::info!("VirtIO balloon device initialized");
        Ok(balloon)
    }

    /// Read device configuration
    fn read_config(virtio: &mut Virtio) -> AgaveResult<VirtioBalloonConfig> {
        let num_pages = virtio.read_config_u32(0)?;
        let actual = virtio.read_config_u32(4)?;
        let free_page_hint_cmd_id = virtio.read_config_u32(8).unwrap_or(0);
        let poison_val = virtio.read_config_u32(12).unwrap_or(0);

        Ok(VirtioBalloonConfig {
            num_pages,
            actual,
            free_page_hint_cmd_id,
            poison_val,
        })
    }

    /// Set up balloon queues
    fn setup_queues(&mut self) -> AgaveResult<()> {
        // Set up statistics queue if enabled
        if self.stats_enabled {
            self.virtio.queue_select(BALLOON_STATS_QUEUE);
            if let Some(desc_id) = self.virtio.get_free_desc_id() {
                self.virtio.set_writable_available(desc_id);
            }
        }

        log::debug!("Balloon queues set up");
        Ok(())
    }

    /// Inflate balloon by allocating and pinning pages
    pub fn inflate(&mut self, num_pages: u32) -> AgaveResult<u32> {
        if num_pages == 0 {
            return Ok(0);
        }

        log::info!("Inflating balloon by {} pages", num_pages);

        let pages_to_inflate = num_pages.min(MAX_PAGES_PER_OPERATION as u32);
        let mut page_addresses = Vec::with_capacity(pages_to_inflate as usize);
        let mut allocated_pages = Vec::new();

        // Allocate physical pages
        {
            let mut frame_allocator = FRAME_ALLOCATOR.get().unwrap().lock();

            for _ in 0..pages_to_inflate {
                if let Some(frame) = frame_allocator.allocate_frame() {
                    let page = BalloonPage {
                        frame,
                        address: frame.start_address(),
                    };
                    page_addresses.push(page.address.as_u64());
                    allocated_pages.push(page);
                } else {
                    log::warn!("Failed to allocate page for balloon inflation");
                    break;
                }
            }
        }

        let actual_pages = allocated_pages.len() as u32;
        if actual_pages == 0 {
            return Err(AgaveError::OutOfMemory);
        }

        // Send pages to host via inflate queue
        self.send_pages_to_host(BALLOON_INFLATE_QUEUE, &page_addresses)?;

        // Store inflated pages
        self.inflated_pages.extend(allocated_pages);

        // Update statistics
        self.stats.pages_inflated += actual_pages as u64;
        self.stats.current_pages += actual_pages;

        // Update device configuration
        self.config.actual = self.stats.current_pages;
        self.virtio.write_config_u32(4, self.config.actual)?;

        log::info!(
            "Inflated {} pages, total balloon size: {} pages",
            actual_pages,
            self.stats.current_pages
        );

        Ok(actual_pages)
    }

    /// Deflate balloon by releasing pages back to the system
    pub fn deflate(&mut self, num_pages: u32) -> AgaveResult<u32> {
        if num_pages == 0 || self.inflated_pages.is_empty() {
            return Ok(0);
        }

        log::info!("Deflating balloon by {} pages", num_pages);

        let pages_to_deflate = num_pages.min(self.inflated_pages.len() as u32);
        let mut page_addresses = Vec::with_capacity(pages_to_deflate as usize);
        let mut pages_to_free = Vec::new();

        // Collect pages to deflate
        for _ in 0..pages_to_deflate {
            if let Some(page) = self.inflated_pages.pop() {
                page_addresses.push(page.address.as_u64());
                pages_to_free.push(page);
            }
        }

        let actual_pages = pages_to_free.len() as u32;
        if actual_pages == 0 {
            return Ok(0);
        }

        // Tell host about deflation
        self.send_pages_to_host(BALLOON_DEFLATE_QUEUE, &page_addresses)?;

        // Return pages to frame allocator
        {
            // let mut frame_allocator = FRAME_ALLOCATOR.get().unwrap().lock();
            for page in pages_to_free {
                // Note: BootInfoFrameAllocator doesn't support deallocation
                // This would need a proper frame deallocator implementation
                // frame_allocator.deallocate_frame(page.frame);
                log::debug!("Would deallocate frame at {:?}", page.frame);
            }
        }

        // Update statistics
        self.stats.pages_deflated += actual_pages as u64;
        self.stats.current_pages -= actual_pages;

        // Update device configuration
        self.config.actual = self.stats.current_pages;
        self.virtio.write_config_u32(4, self.config.actual)?;

        log::info!(
            "Deflated {} pages, total balloon size: {} pages",
            actual_pages,
            self.stats.current_pages
        );

        Ok(actual_pages)
    }

    /// Send page addresses to host
    fn send_pages_to_host(&mut self, queue_id: u16, page_addresses: &[u64]) -> AgaveResult<()> {
        self.virtio.queue_select(queue_id);

        // Create buffer with page frame numbers (divided by page size)
        let mut pfns = Vec::with_capacity(page_addresses.len());
        for &addr in page_addresses {
            pfns.push((addr / BALLOON_PAGE_SIZE as u64) as u32);
        }

        if let Some((desc_id, desc_next_id)) = self.virtio.get_free_twice_desc_id() {
            // Set up descriptor for PFN array
            unsafe {
                let descs = self.virtio.common.cap.queue_desc as *mut Desc;
                let mut desc = descs.offset(desc_id as isize).read_volatile();

                desc.addr = pfns.as_ptr() as u64;
                desc.len = (pfns.len() * core::mem::size_of::<u32>()) as u32;
                desc.flags = 1; // VIRTQ_DESC_F_NEXT
                desc.next = desc_next_id;

                descs.offset(desc_id as isize).write_volatile(desc);
            }

            self.virtio.set_writable(desc_next_id);
            self.virtio.set_available(desc_id);
            self.virtio.kick(queue_id);

            // TODO: Wait for completion properly
            self.virtio.set_free_desc_id(desc_id);
            self.virtio.set_free_desc_id(desc_next_id);
        } else {
            return Err(AgaveError::ResourceExhausted);
        }

        Ok(())
    }

    /// Send balloon statistics to host
    pub fn send_statistics(&mut self) -> AgaveResult<()> {
        if !self.stats_enabled {
            return Err(AgaveError::NotImplemented);
        }

        // Update memory statistics
        self.update_memory_stats()?;

        // Create statistics buffer
        let mut stats_buffer = Vec::new();

        stats_buffer.push(VirtioBalloonStat {
            tag: BalloonStatTag::FreeMemory as u16,
            val: self.stats.free_memory,
        });

        stats_buffer.push(VirtioBalloonStat {
            tag: BalloonStatTag::TotalMemory as u16,
            val: self.stats.total_memory,
        });

        stats_buffer.push(VirtioBalloonStat {
            tag: BalloonStatTag::AvailableMemory as u16,
            val: self.stats.available_memory,
        });

        stats_buffer.push(VirtioBalloonStat {
            tag: BalloonStatTag::MajorFaults as u16,
            val: self.stats.major_faults,
        });

        stats_buffer.push(VirtioBalloonStat {
            tag: BalloonStatTag::MinorFaults as u16,
            val: self.stats.minor_faults,
        });

        // Send statistics via stats queue
        self.virtio.queue_select(BALLOON_STATS_QUEUE);

        if let Some((desc_id, desc_next_id)) = self.virtio.get_free_twice_desc_id() {
            unsafe {
                let descs = self.virtio.common.cap.queue_desc as *mut Desc;
                let mut desc = descs.offset(desc_id as isize).read_volatile();

                desc.addr = stats_buffer.as_ptr() as u64;
                desc.len = (stats_buffer.len() * core::mem::size_of::<VirtioBalloonStat>()) as u32;
                desc.flags = 1; // VIRTQ_DESC_F_NEXT
                desc.next = desc_next_id;

                descs.offset(desc_id as isize).write_volatile(desc);
            }

            self.virtio.set_writable(desc_next_id);
            self.virtio.set_available(desc_id);
            self.virtio.kick(BALLOON_STATS_QUEUE);

            // TODO: Wait for completion properly
            self.virtio.set_free_desc_id(desc_id);
            self.virtio.set_free_desc_id(desc_next_id);
        } else {
            return Err(AgaveError::ResourceExhausted);
        }

        log::debug!("Sent balloon statistics to host");
        Ok(())
    }

    /// Update memory statistics from system
    fn update_memory_stats(&mut self) -> AgaveResult<()> {
        // In a real implementation, this would gather actual memory statistics
        // For now, provide placeholder values

        self.stats.total_memory = 1024 * 1024 * 1024; // 1GB placeholder
        self.stats.free_memory = 512 * 1024 * 1024; // 512MB placeholder
        self.stats.available_memory = self.stats.free_memory;

        // Increment fault counters (placeholders)
        self.stats.major_faults += 1;
        self.stats.minor_faults += 10;

        Ok(())
    }

    /// Process balloon requests from host
    pub fn process_requests(&mut self) -> AgaveResult<()> {
        // Check for configuration changes
        let new_config = Self::read_config(&mut self.virtio)?;

        let current_num_pages = unsafe {
            core::ptr::read_unaligned(
                (&self.config as *const VirtioBalloonConfig as *const u8).add(0) as *const u32,
            )
        };
        if new_config.num_pages != current_num_pages {
            let new_num_pages = unsafe {
                core::ptr::read_unaligned(
                    (&new_config as *const VirtioBalloonConfig as *const u8).add(0) as *const u32,
                )
            };
            log::info!(
                "Balloon target changed: {} -> {} pages",
                current_num_pages,
                new_num_pages
            );

            unsafe {
                core::ptr::write_unaligned(
                    (&mut self.config as *mut VirtioBalloonConfig as *mut u8).add(0) as *mut u32,
                    new_config.num_pages,
                )
            };
            self.stats.target_pages = new_config.num_pages;

            // Adjust balloon size to match target
            self.adjust_to_target()?;
        }

        Ok(())
    }

    /// Adjust balloon size to match target
    fn adjust_to_target(&mut self) -> AgaveResult<()> {
        let current = self.stats.current_pages;
        let target = self.stats.target_pages;

        if target > current {
            // Need to inflate
            let pages_to_inflate = target - current;
            self.inflate(pages_to_inflate)?;
        } else if target < current {
            // Need to deflate
            let pages_to_deflate = current - target;
            self.deflate(pages_to_deflate)?;
        }

        Ok(())
    }

    /// Emergency deflation for out-of-memory situations
    pub fn emergency_deflate(&mut self, pages_needed: u32) -> AgaveResult<u32> {
        if !self.deflate_on_oom {
            return Err(AgaveError::NotImplemented);
        }

        log::warn!("Emergency balloon deflation: {} pages needed", pages_needed);

        let pages_to_deflate = pages_needed.min(self.inflated_pages.len() as u32);
        self.deflate(pages_to_deflate)
    }

    /// Get current balloon statistics
    pub fn get_stats(&self) -> &BalloonStats {
        &self.stats
    }

    /// Get current balloon size in pages
    pub fn current_size(&self) -> u32 {
        self.stats.current_pages
    }

    /// Get target balloon size in pages
    pub fn target_size(&self) -> u32 {
        self.stats.target_pages
    }

    /// Check if balloon can deflate on OOM
    pub fn can_deflate_on_oom(&self) -> bool {
        self.deflate_on_oom
    }

    /// Check if page reporting is supported
    pub fn supports_page_reporting(&self) -> bool {
        self.page_reporting
    }
}

/// Global balloon device instance
pub static VIRTIO_BALLOON: Mutex<Option<VirtioBalloon>> = Mutex::new(None);

/// Public driver function
pub async fn drive(virtio: Virtio) {
    log::info!("Starting VirtIO memory balloon driver");

    let balloon = match VirtioBalloon::new(virtio) {
        Ok(device) => device,
        Err(e) => {
            log::error!("Failed to initialize VirtIO balloon: {:?}", e);
            return;
        }
    };

    // Store in global instance
    *VIRTIO_BALLOON.lock() = Some(balloon);

    log::info!("VirtIO memory balloon driver ready");

    // Main driver loop
    loop {
        // Process balloon requests and statistics
        if let Some(ref mut balloon) = VIRTIO_BALLOON.lock().as_mut() {
            if let Err(e) = balloon.process_requests() {
                log::error!("Error processing balloon requests: {:?}", e);
            }

            // Send statistics periodically (every 30 seconds)
            let now = crate::sys::interrupts::global_time_ms();
            let last_update = balloon.last_stats_update.load(Ordering::Relaxed);

            if now.saturating_sub(last_update) > 30000 {
                if let Err(e) = balloon.send_statistics() {
                    log::debug!("Error sending balloon statistics: {:?}", e);
                } else {
                    balloon.last_stats_update.store(now, Ordering::Relaxed);
                }
            }
        }

        yield_once().await;
    }
}

/// Public API functions

/// Inflate balloon by specified number of pages
pub fn inflate_balloon(pages: u32) -> AgaveResult<u32> {
    if let Some(ref mut balloon) = VIRTIO_BALLOON.lock().as_mut() {
        balloon.inflate(pages)
    } else {
        Err(AgaveError::NotReady)
    }
}

/// Deflate balloon by specified number of pages
pub fn deflate_balloon(pages: u32) -> AgaveResult<u32> {
    if let Some(ref mut balloon) = VIRTIO_BALLOON.lock().as_mut() {
        balloon.deflate(pages)
    } else {
        Err(AgaveError::NotReady)
    }
}

/// Emergency deflation for OOM situations
pub fn emergency_deflate_balloon(pages_needed: u32) -> AgaveResult<u32> {
    if let Some(ref mut balloon) = VIRTIO_BALLOON.lock().as_mut() {
        balloon.emergency_deflate(pages_needed)
    } else {
        Err(AgaveError::NotReady)
    }
}

/// Get balloon statistics
pub fn get_balloon_stats() -> Option<BalloonStats> {
    VIRTIO_BALLOON
        .lock()
        .as_ref()
        .map(|balloon| balloon.get_stats().clone())
}

/// Get current balloon size
pub fn get_balloon_size() -> Option<u32> {
    VIRTIO_BALLOON
        .lock()
        .as_ref()
        .map(|balloon| balloon.current_size())
}

/// Check if balloon is available
pub fn is_balloon_available() -> bool {
    VIRTIO_BALLOON.lock().is_some()
}

/// Integration with memory allocator for OOM handling
pub fn handle_oom(pages_needed: u32) -> bool {
    if let Ok(freed_pages) = emergency_deflate_balloon(pages_needed) {
        log::info!("OOM: freed {} pages from balloon", freed_pages);
        freed_pages > 0
    } else {
        false
    }
}
