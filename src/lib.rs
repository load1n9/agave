#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]
#![feature(try_blocks)]
#![no_std]

extern crate alloc;

pub mod sys;
pub mod api;
pub mod vga;

/// Initialize the kernel
pub fn init() {
    sys::gdt::init();
    sys::interrupts::init_idt();
    unsafe { sys::interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
}

pub fn halt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
