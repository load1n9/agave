#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]
#![feature(try_blocks)]
#![no_std]

extern crate alloc;

pub mod allocator;
pub mod exit;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod serial;
pub mod task;
pub mod vga_buffer;
pub mod wasm;

/// Initialize the kernel
pub fn init() {
    gdt::init();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
}

pub fn halt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
