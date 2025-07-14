// WASI Preview 1 (legacy) implementation for Agave OS
// This provides the original WASI snapshot_preview1 API for compatibility

use super::error::*;
use super::types::*;
use super::{cli, clocks, filesystem, io, random, sockets};
use alloc::vec::Vec;
use wasmi::{Caller, Func, Linker, Store};

pub fn link_preview1_functions<T>(
    linker: &mut Linker<T>,
    _store: &mut Store<T>,
) -> Result<(), wasmi::Error>
where
    T: 'static,
{
    // Since we need to create many functions and can't borrow store multiple times,
    // we'll use a simpler approach by defining all functions inline

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "args_get",
        |_caller: Caller<'_, T>, argv: i32, argv_buf: i32| -> i32 {
            log::debug!("args_get({}, {})", argv, argv_buf);
            match cli::args_get(argv as u32, argv_buf as u32) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "args_sizes_get",
        |_caller: Caller<'_, T>, argc_ptr: i32, argv_buf_size_ptr: i32| -> i32 {
            log::debug!("args_sizes_get({}, {})", argc_ptr, argv_buf_size_ptr);
            match cli::args_sizes_get() {
                Ok((_argc, _argv_buf_size)) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "environ_get",
        |_caller: Caller<'_, T>, environ: i32, environ_buf: i32| -> i32 {
            log::debug!("environ_get({}, {})", environ, environ_buf);
            match cli::environ_get(environ as u32, environ_buf as u32) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "environ_sizes_get",
        |_caller: Caller<'_, T>, environc_ptr: i32, environ_buf_size_ptr: i32| -> i32 {
            log::debug!(
                "environ_sizes_get({}, {})",
                environc_ptr,
                environ_buf_size_ptr
            );
            match cli::environ_sizes_get() {
                Ok((_environc, _environ_buf_size)) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "clock_res_get",
        |_caller: Caller<'_, T>, id: i32, resolution_ptr: i32| -> i32 {
            log::debug!("clock_res_get({}, {})", id, resolution_ptr);
            match clocks::clock_res_get(id as Clockid) {
                Ok(_resolution) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "clock_time_get",
        |_caller: Caller<'_, T>, id: i32, precision: i64, time_ptr: i32| -> i32 {
            log::debug!("clock_time_get({}, {}, {})", id, precision, time_ptr);
            match clocks::clock_time_get(id as Clockid, precision as Timestamp) {
                Ok(_time) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "random_get",
        |_caller: Caller<'_, T>, buf: i32, buf_len: i32| -> i32 {
            log::debug!("random_get({}, {})", buf, buf_len);
            match random::get_random_bytes(buf_len as u64) {
                Ok(_data) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_close",
        |_caller: Caller<'_, T>, fd: i32| -> i32 {
            log::debug!("fd_close({})", fd);
            match filesystem::fd_close(fd as Fd) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_write",
        |_caller: Caller<'_, T>, fd: i32, iovs: i32, iovs_len: i32, nwritten_ptr: i32| -> i32 {
            log::debug!("fd_write({}, {}, {}, {})", fd, iovs, iovs_len, nwritten_ptr);
            // In a real implementation, write iovs to file
            ERRNO_SUCCESS as i32
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_read",
        |_caller: Caller<'_, T>, fd: i32, iovs: i32, iovs_len: i32, nread_ptr: i32| -> i32 {
            log::debug!("fd_read({}, {}, {}, {})", fd, iovs, iovs_len, nread_ptr);
            // In a real implementation, read from file and write to iovs
            ERRNO_SUCCESS as i32
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "proc_exit",
        |_caller: Caller<'_, T>, exit_code: i32| {
            log::info!("proc_exit({})", exit_code);
            cli::proc_exit(exit_code as ExitCode);
        },
    )?;

    // Add more essential WASI functions as needed
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_fdstat_get",
        |_caller: Caller<'_, T>, fd: i32, stat_ptr: i32| -> i32 {
            log::debug!("fd_fdstat_get({}, {})", fd, stat_ptr);
            ERRNO_SUCCESS as i32
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_prestat_get",
        |_caller: Caller<'_, T>, fd: i32, prestat_ptr: i32| -> i32 {
            log::debug!("fd_prestat_get({}, {})", fd, prestat_ptr);
            ERRNO_SUCCESS as i32
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_prestat_dir_name",
        |_caller: Caller<'_, T>, fd: i32, path: i32, path_len: i32| -> i32 {
            log::debug!("fd_prestat_dir_name({}, {}, {})", fd, path, path_len);
            ERRNO_SUCCESS as i32
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "sched_yield",
        |_caller: Caller<'_, T>| -> i32 {
            log::debug!("sched_yield()");
            ERRNO_SUCCESS as i32
        },
    )?;

    Ok(())
}
