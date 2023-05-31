#![no_std]
#![no_main]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use agave_os::{sys, usr, print, println, debug, hlt_loop};

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    agave_os::init(boot_info);
    print!("\x1b[?25h"); // Enable cursor
    loop {
        if let Some(cmd) = option_env!("agave_os_CMD") {
            let prompt = usr::shell::prompt_string(true);
            println!("{}{}", prompt, cmd);
            usr::shell::exec(cmd).ok();
            sys::acpi::shutdown();
        } else {
            user_boot();
        }
    }
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
        println!("Running console in diskless mode");
        usr::shell::main(&["shell"]).ok();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    debug!("{}", info);
    hlt_loop();
}
