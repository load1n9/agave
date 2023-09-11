#![no_std]
#![feature(core_intrinsics)]
#![feature(abi_x86_interrupt)]
#![feature(slice_first_last_chunk)]
#![feature(strict_provenance)]
#![feature(allocator_api)]

extern crate alloc;
extern crate lazy_static;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

pub fn hlt_loop() -> ! {
    loop {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        x86_64::instructions::hlt();
    }
}

pub mod path;
pub mod sys;
