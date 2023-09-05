use pic8259::ChainedPics;
use spin::Mutex;

pub const PIC_1_OFFSET: u8 = 32;

pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub const IRQ_BASE_OFFSET: u8 = 0x20;

pub const PIC_SPURIOUS_INTERRUPT_IRQ: u8 = IRQ_BASE_OFFSET + 0x7;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

pub fn init() {
    unsafe {
        PICS.lock().initialize();
    }
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    x86_64::instructions::interrupts::enable();
}
