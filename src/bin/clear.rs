#![no_std]
#![no_main]

use agave_kernel::api::syscall;
use agave_kernel::entry_point;

entry_point!(main);

fn main(_args: &[&str]) {
    syscall::write(1, b"\x1b[2J\x1b[1;1H"); // Clear screen and move cursor to top
}
