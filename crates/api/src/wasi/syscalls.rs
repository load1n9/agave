use wasi::{self, Exitcode};

use crate::syscall::exit;

/// Terminate the process normally.
/// An exit code of 0 indicates successful termination of the program.
/// The meanings of other values is dependent on the environment.
pub fn proc_exit(code: Exitcode) {
    let status = match code {
        0 => crate::process::ExitCode::Success,
        _ => crate::process::ExitCode::Failure,
    };

    exit(status);
}