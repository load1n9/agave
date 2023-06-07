#![no_std]
#![no_main]

use agave_kernel::api::process::ExitCode;
use agave_kernel::api::syscall;
use agave_kernel::entry_point;

entry_point!(main);

fn main(args: &[&str]) {
    if args.len() == 2 {
        if let Ok(duration) = args[1].parse::<f64>() {
            syscall::sleep(duration);
        } else {
            syscall::exit(ExitCode::DataError);
        }
    } else {
        syscall::exit(ExitCode::UsageError);
    }
}
