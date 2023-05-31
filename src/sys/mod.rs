#[macro_export]
macro_rules! printk {
    ($($arg:tt)*) => ({
        $crate::sys::console::print_fmt(format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ({
        let csi_color = $crate::api::console::Style::color("LightBlue");
        let csi_reset = $crate::api::console::Style::reset();
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
            let csi_color = $crate::api::console::Style::color("LightGreen");
            let csi_reset = $crate::api::console::Style::reset();
            $crate::sys::console::print_fmt(format_args!("{}[{:.6}]{} ", csi_color, uptime, csi_reset));
            $crate::sys::console::print_fmt(format_args!($($arg)*));
            // TODO: Add newline
        }
    });
}

pub mod acpi;
pub mod allocator;
pub mod ata;
pub mod clock;
pub mod cmos;
pub mod console;
pub mod cpu;
pub mod fs;
pub mod gdt;
pub mod idt;
pub mod keyboard;
pub mod mem;
pub mod net;
pub mod pci;
pub mod pic;
pub mod process;
pub mod random;
pub mod serial;
pub mod syscall;
pub mod time;
pub mod vga;
