#![no_std]
#![no_main]

extern crate agave_os;
use agave_os::println;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello World{}", "!");

    agave_os::init();

    println!("It did not crash!");
    agave_os::halt_loop();
}
