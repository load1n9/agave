pub mod allocator;
pub mod drivers;
pub mod framebuffer;
pub mod gdt;
pub mod globals;
pub mod interrupts;
pub mod ioapic;
pub mod local_apic;
pub mod logger;
pub mod memory;
pub mod pci;
pub mod serial;
pub mod task;
pub mod virtio;

use self::{allocator::ALLOCATOR, memory::BootInfoFrameAllocator};
use acpi::{AcpiHandler, PhysicalMapping};
use conquer_once::spin::OnceCell;
use core::{
    alloc::GlobalAlloc,
    ptr::NonNull,
    sync::atomic::{AtomicU64, Ordering},
};
use spin::Mutex;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags,
        PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

extern "C" fn _log_fn(s: *const u8, l: u32) {
    unsafe {
        let slice = core::slice::from_raw_parts(s, l as usize);
        let str_slice = core::str::from_utf8_unchecked(slice);
        log::info!("{}", str_slice)
    }
}

extern "C" fn _calloc(size: usize, align: usize) -> *mut u8 {
    // log::info!("alloc {} {}", size, align);
    unsafe { ALLOCATOR.alloc(core::alloc::Layout::from_size_align(size, align).unwrap()) }
}
extern "C" fn _cdalloc(ptr: *mut u8, size: usize, align: usize) {
    // log::info!("dealloc {:?} {} {}", ptr, size, align);
    unsafe {
        ALLOCATOR.dealloc(
            ptr,
            core::alloc::Layout::from_size_align(size, align).unwrap(),
        );
    };
}

#[derive(Clone)]
pub struct AcpiHandlerImpl;
impl AcpiHandler for AcpiHandlerImpl {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        let s = (size / 4096 + 1) * 4096;
        PhysicalMapping::new(
            physical_address,
            NonNull::new(phys_to_virt(PhysAddr::new(physical_address as u64)).as_mut_ptr())
                .unwrap(),
            s,
            s,
            self.clone(),
        )
    }
    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {}
}
const _ACPI_HANDLER: AcpiHandlerImpl = AcpiHandlerImpl;

pub static MAPPER: OnceCell<Mutex<OffsetPageTable>> = OnceCell::uninit();
pub static FRAME_ALLOCATOR: OnceCell<Mutex<BootInfoFrameAllocator>> = OnceCell::uninit();

pub static mut VIRTUAL_MAPPING_OFFSET: VirtAddr = VirtAddr::new_truncate(0);
pub fn phys_to_virt(addr: PhysAddr) -> VirtAddr {
    unsafe { VIRTUAL_MAPPING_OFFSET + addr.as_u64() }
}
static OTHER_VIRT: AtomicU64 = AtomicU64::new(0x_5000_0000_0000);
pub fn create_virt_from_phys(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    frame: PhysFrame,
) -> Result<Page, MapToError<Size4KiB>> {
    let start = VirtAddr::new(OTHER_VIRT.fetch_add(4096, Ordering::Relaxed) as u64);
    let page = Page::containing_address(start);
    let flags =
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
    unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    return Ok(page);
}

pub fn create_identity_virt_from_phys(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<Page, MapToError<Size4KiB>> {
    let frame = frame_allocator.allocate_frame().unwrap();
    let start = VirtAddr::new(frame.start_address().as_u64());
    let page = Page::containing_address(start);
    let flags =
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
    unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    return Ok(page);
}

pub fn with_mapper_framealloc<FUNC, R>(f: FUNC) -> R
where
    FUNC: FnOnce(&mut OffsetPageTable, &mut BootInfoFrameAllocator) -> R,
{
    let mut mapper = MAPPER.get().unwrap().lock();
    let mut frame_allocator = FRAME_ALLOCATOR.get().unwrap().lock();
    let mapper = &mut *mapper;
    let frame_allocator = &mut *frame_allocator;
    f(mapper, frame_allocator)
}

pub fn create_identity_virt_from_phys_n(pages: usize) -> Result<Page, MapToError<Size4KiB>> {
    with_mapper_framealloc(|mapper, frame_allocator| {
        let first_frame = frame_allocator.allocate_frame().unwrap();
        log::info!("first_frame {}", first_frame.start_address().as_u64());
        for i in 1..pages {
            let frame = frame_allocator.allocate_frame().unwrap();
            let frame_start = frame.start_address().as_u64();

            // log::info!("{} : {}", i, frame_start);
            if first_frame.start_address().as_u64() + (i as u64) * 4096 != frame_start {
                panic!("create_identity_virt_from_phys_n NON CONTIGUOUS, {}", i)
            }
        }

        for i in 0..pages {
            let addr = first_frame.start_address().as_u64() + (i as u64) * 4096;
            let frame = PhysFrame::containing_address(PhysAddr::new(addr));
            let page = Page::containing_address(VirtAddr::new(addr));
            let flags = PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::USER_ACCESSIBLE;
            unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
        }

        return Ok(Page::containing_address(VirtAddr::new(
            first_frame.start_address().as_u64(),
        )));
    })
}

pub const ACPI_HANDLER: AcpiHandlerImpl = AcpiHandlerImpl;