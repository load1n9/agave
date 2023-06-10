#![no_std]
#![no_main]

extern crate alloc;

use agave_kernel::api::syscall;
use agave_kernel::entry_point;

entry_point!(main);

fn main(_args: &[&str]) {
    syscall::write(1, b"Hello from Agave!\n");
}
