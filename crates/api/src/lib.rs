#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(doc_cfg)]
#![feature(error_in_core)]
#![feature(step_trait)]
#![feature(alloc_error_handler)]
#![feature(naked_functions)]
#![feature(never_type)]
#![feature(exact_size_is_empty)]
#![feature(custom_test_frameworks)]
#![feature(ptr_internals)]
extern crate alloc;
extern crate lazy_static;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[macro_export]
macro_rules! printk {
    ($($arg:tt)*) => ({
        $crate::sys::console::print_fmt(format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ({
        let csi_color = $crate::console::Style::color("LightBlue");
        let csi_reset = $crate::console::Style::reset();
        $crate::sys::console::print_fmt(format_args!("{}DEBUG: ", csi_color));
        $crate::sys::console::print_fmt(format_args!($($arg)*));
        $crate::sys::console::print_fmt(format_args!("{}\n", csi_reset));
    });
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => ({
        if !cfg!(test) {
            let uptime = $crate::sys::clock::uptime();
            let csi_color = $crate::console::Style::color("LightGreen");
            let csi_reset = $crate::console::Style::reset();
            $crate::sys::console::print_fmt(format_args!("{}[{:.6}]{} ", csi_color, uptime, csi_reset));
            $crate::sys::console::print_fmt(format_args!($($arg)*));
            // TODO: Add newline
        }
    });
}

#[macro_export]
macro_rules! entry_point {
    ($path:path) => {
        #[panic_handler]
        fn panic(_info: &core::panic::PanicInfo) -> ! {
            $crate::syscall::write(1, b"An exception occured!\n");
            loop {}
        }

        #[export_name = "_start"]
        pub unsafe extern "sysv64" fn __impl_start(args_ptr: u64, args_len: usize) {
            let args = core::slice::from_raw_parts(args_ptr as *const _, args_len);
            let f: fn(&[&str]) = $path;
            f(args);
            $crate::syscall::exit($crate::process::ExitCode::Success);
        }
    };
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use alloc::format;
        let s = format!("{}", format_args!($($arg)*));
        $crate::io::stdout().write(&s);
    });
}

#[macro_export]
macro_rules! println {
    () => ({
        print!("\n");
    });
    ($($arg:tt)*) => ({
        print!("{}\n", format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => ({
        use alloc::format;
        let s = format!("{}", format_args!($($arg)*));
        $crate::api::io::stderr().write(&s);
    });
}

#[macro_export]
macro_rules! eprintln {
    () => ({
        eprint!("\n");
    });
    ($($arg:tt)*) => ({
        eprint!("{}\n", format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ({
        let csi_color = $crate::console::Style::color("LightRed");
        let csi_reset = $crate::console::Style::reset();
        eprintln!("{}Error:{} {}", csi_color, csi_reset, format_args!($($arg)*));
    });
}

pub mod clock;
pub mod config;
pub mod console;
pub mod font;
pub mod fs;
pub mod io;
pub mod path;
pub mod process;
pub mod realm;
pub mod sys;
pub mod syscall;
pub mod vga;
pub mod wasi;
pub mod wasm;