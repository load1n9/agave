#![no_std]
#![no_main]
extern crate agave_kernel;
extern crate alloc;

use agave_kernel::{debug, hlt_loop, print, println, sys, usr, init_logger};
use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use core::panic::PanicInfo;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(main, config = &BOOTLOADER_CONFIG);

fn main(boot_info: &'static mut BootInfo) -> ! {
    // agave_kernel::init(boot_info);
    // print!("\x1b[?25h");
    // loop {
    //     if let Some(cmd) = option_env!("agave_os_CMD") {
    //         let prompt = usr::shell::prompt_string(true);
    //         println!("{}{}", prompt, cmd);
    //         usr::shell::exec(cmd).ok();
    //         sys::acpi::shutdown();
    //     } else {
    //         user_boot();
    //     }
    // }
    if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
        let mut value = 0x90;
        for byte in framebuffer.buffer_mut() {
            *byte = value;
            value = value.wrapping_add(1);
        }
    }
    let frame_buffer_optional = &mut boot_info.framebuffer;
    let frame_buffer_option = frame_buffer_optional.as_mut();
    let frame_buffer_struct = frame_buffer_option.unwrap();
    let frame_buffer_info = frame_buffer_struct.info().clone();
    let raw_frame_buffer = frame_buffer_struct.buffer_mut();
    init_logger(raw_frame_buffer, frame_buffer_info);
    loop {}
}

fn user_boot() {
    let script = "/ini/boot.sh";
    if sys::fs::File::open(script).is_some() {
        usr::shell::main(&["shell", script]).ok();
    } else {
        if sys::fs::is_mounted() {
            println!("Could not find '{}'", script);
        } else {
            println!("MFS is not mounted to '/'");
        }
        println!("Running in diskless mode type `install` to install to disk");
        sys::fs::mount_mem();
        sys::fs::format_mem();
        usr::shell::main(&["shell"]).ok();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    debug!("{}", info);
    hlt_loop();
}
