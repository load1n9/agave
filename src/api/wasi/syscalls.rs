use crate::api::process::ExitCode;

pub fn proc_exit(code: ExitCode) {
   crate::api::syscall::exit(code);
}
