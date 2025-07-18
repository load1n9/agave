// WASI Preview 1 (legacy) implementation for Agave OS
// This provides the original WASI snapshot_preview1 API for compatibility

use super::types::*;
use super::{cli, /*clocks,*/ filesystem, random};
use wasmi::{Caller, Linker, Store};

#[allow(dependency_on_unit_never_type_fallback)]
pub fn link_preview1_functions<T>(
    linker: &mut Linker<T>,
    _store: &mut Store<T>,
) -> Result<(), wasmi::Error>
where
    T: 'static,
{
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "environ_get",
        |_caller: Caller<'_, T>, environ: i32, environ_buf: i32| -> i32 {
            log::debug!("environ_get({}, {})", environ, environ_buf);
            // For demo, use a static environment list
            let envs = ["PATH=/usr/bin", "HOME=/root", "USER=agave"];
            let mut buf_offset = 0;
            let mut ptr_offset = 0;
            unsafe {
                let environ_ptrs = environ as *mut i32;
                let environ_buf_ptr = environ_buf as *mut u8;
                for env in envs.iter() {
                    let bytes = env.as_bytes();
                    core::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        environ_buf_ptr.add(buf_offset),
                        bytes.len(),
                    );
                    *environ_ptrs.add(ptr_offset) = environ_buf + buf_offset as i32;
                    buf_offset += bytes.len();
                    // Null terminator
                    *environ_buf_ptr.add(buf_offset) = 0;
                    buf_offset += 1;
                    ptr_offset += 1;
                }
            }
            ERRNO_SUCCESS as i32
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "environ_sizes_get",
        |_caller: Caller<'_, T>, count_ptr: i32, buf_size_ptr: i32| -> i32 {
            log::debug!("environ_sizes_get({}, {})", count_ptr, buf_size_ptr);
            let envs = ["PATH=/usr/bin", "HOME=/root", "USER=agave"];
            let count = envs.len() as u32;
            let buf_size = envs.iter().map(|e| e.len() + 1).sum::<usize>() as u32;
            unsafe {
                *(count_ptr as *mut u32) = count;
                *(buf_size_ptr as *mut u32) = buf_size;
            }
            ERRNO_SUCCESS as i32
        },
    )?;


    // Register path_create_directory (directory creation)
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_create_directory",
        |_caller: Caller<'_, T>, fd: i32, path_ptr: i32, path_len: i32| -> i32 {
            log::debug!("path_create_directory({}, {}, {})", fd, path_ptr, path_len);
            // Safety: path_ptr is a pointer to guest memory
            let path_bytes = unsafe {
                core::slice::from_raw_parts(path_ptr as *const u8, path_len as usize)
            };
            match core::str::from_utf8(path_bytes) {
                Ok(path_str) => match filesystem::path_create_directory(fd as Fd, path_str) {
                    Ok(()) => ERRNO_SUCCESS as i32,
                    Err(e) => e.errno as i32,
                },
                Err(_) => ERRNO_INVAL as i32,
            }
        },
    )?;
    // ...existing code...
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
        "fd_read",
        |_caller: Caller<'_, T>, fd: i32, iovs: i32, iovs_len: i32, nread_ptr: i32| -> i32 {
            log::debug!("fd_read({}, {}, {}, {})", fd, iovs, iovs_len, nread_ptr);
            let iovec = IOVec {
                buf: iovs as u32,
                buf_len: iovs_len as u32,
            };
            match filesystem::fd_read(fd as Fd, &[iovec]) {
                Ok(nread) => {
                    unsafe {
                        let nread_ptr = nread_ptr as *mut u32;
                        *nread_ptr = nread;
                    }
                    ERRNO_SUCCESS as i32
                }
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
            let iovec = IOVec {
                buf: iovs as u32,
                buf_len: iovs_len as u32,
            };
            match filesystem::fd_write(fd as Fd, &[iovec]) {
                Ok(nwritten) => {
                    // Write nwritten to nwritten_ptr in WebAssembly memory
                    unsafe {
                        let nwritten_ptr = nwritten_ptr as *mut u32;
                        *nwritten_ptr = nwritten;
                    }
                    ERRNO_SUCCESS as i32
                }
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_readdir",
        |_caller: Caller<'_, T>,
         fd: i32,
         buf: i32,
         buf_len: i32,
         cookie: i64,
         bufused_ptr: i32|
         -> i32 {
            log::debug!(
                "fd_readdir({}, {}, {}, {}, {})",
                fd,
                buf,
                buf_len,
                cookie,
                bufused_ptr
            );
            let fd = fd as Fd;
            let cookie = cookie as DirCookie;
            let buf_slice =
                unsafe { core::slice::from_raw_parts_mut(buf as *mut u8, buf_len as usize) };
            match filesystem::fd_readdir(fd, buf_slice, cookie) {
                Ok(bytes_used) => {
                    unsafe {
                        let bufused_ptr = bufused_ptr as *mut u32;
                        *bufused_ptr = bytes_used as u32;
                    }
                    0 // ERRNO_SUCCESS
                }
                Err(e) => e.errno as i32,
            }
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
            match filesystem::fd_fdstat_get(fd as Fd) {
                Ok(fdstat) => {
                    // Write fdstat to stat_ptr in WebAssembly memory
                    unsafe {
                        let stat_ptr = stat_ptr as *mut FdStat;
                        *stat_ptr = fdstat;
                    }
                    ERRNO_SUCCESS as i32
                }
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_prestat_get",
        |_caller: Caller<'_, T>, fd: i32, prestat_ptr: i32| -> i32 {
            log::debug!("fd_prestat_get({}, {})", fd, prestat_ptr);
            match filesystem::fd_prestat_get(fd as Fd) {
                Ok(prestat) => {
                    // Write prestat to prestat_ptr in WebAssembly memory
                    unsafe {
                        let prestat_ptr = prestat_ptr as *mut Prestat;
                        *prestat_ptr = prestat;
                    }
                    ERRNO_SUCCESS as i32
                }
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_prestat_dir_name",
        |_caller: Caller<'_, T>, fd: i32, path: i32, path_len: i32| -> i32 {
            log::debug!("fd_prestat_dir_name({}, {}, {})", fd, path, path_len);
            match filesystem::fd_prestat_dir_name(fd as Fd, path as u32, path_len as Size) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
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

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_filestat_get",
        |_caller: Caller<'_, T>, fd: i32, filestat_ptr: i32| -> i32 {
            log::debug!("fd_filestat_get({}, {})", fd, filestat_ptr);
            match filesystem::fd_filestat_get(fd as Fd) {
                Ok(filestat) => {
                    unsafe {
                        let filestat_ptr = filestat_ptr as *mut FileStat;
                        *filestat_ptr = filestat;
                    }
                    ERRNO_SUCCESS as i32
                }
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_filestat_set_size",
        |_caller: Caller<'_, T>, fd: i32, size: i64| -> i32 {
            log::debug!("fd_filestat_set_size({}, {})", fd, size);
            match filesystem::fd_filestat_set_size(fd as Fd, size as FileSize) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_fdstat_set_flags",
        |_caller: Caller<'_, T>, fd: i32, flags: i32| -> i32 {
            log::debug!("fd_fdstat_set_flags({}, {})", fd, flags);
            match filesystem::fd_fdstat_set_flags(fd as Fd, flags as FdFlags) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_sync",
        |_caller: Caller<'_, T>, fd: i32| -> i32 {
            log::debug!("fd_sync({})", fd);
            match filesystem::fd_sync(fd as Fd) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_datasync",
        |_caller: Caller<'_, T>, fd: i32| -> i32 {
            log::debug!("fd_datasync({})", fd);
            match filesystem::fd_datasync(fd as Fd) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_allocate",
        |_caller: Caller<'_, T>, fd: i32, offset: i64, len: i64| -> i32 {
            log::debug!("fd_allocate({}, {}, {})", fd, offset, len);
            match filesystem::fd_allocate(fd as Fd, offset as FileSize, len as FileSize) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_advise",
        |_caller: Caller<'_, T>, fd: i32, offset: i64, len: i64, advice: i32| -> i32 {
            log::debug!("fd_advise({}, {}, {}, {})", fd, offset, len, advice);
            match filesystem::fd_advise(
                fd as Fd,
                offset as FileSize,
                len as FileSize,
                advice as Advice,
            ) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_tell",
        |_caller: Caller<'_, T>, fd: i32, offset_ptr: i32| -> i32 {
            log::debug!("fd_tell({}, {})", fd, offset_ptr);
            match filesystem::fd_tell(fd as Fd) {
                Ok(offset) => {
                    unsafe {
                        let offset_ptr = offset_ptr as *mut FileSize;
                        *offset_ptr = offset;
                    }
                    ERRNO_SUCCESS as i32
                }
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_seek",
        |_caller: Caller<'_, T>, fd: i32, offset: i64, whence: i32, new_offset_ptr: i32| -> i32 {
            log::debug!(
                "fd_seek({}, {}, {}, {})",
                fd,
                offset,
                whence,
                new_offset_ptr
            );
            match filesystem::fd_seek(fd as Fd, offset as FileDelta, whence as Whence) {
                Ok(new_offset) => {
                    unsafe {
                        let new_offset_ptr = new_offset_ptr as *mut FileSize;
                        *new_offset_ptr = new_offset;
                    }
                    ERRNO_SUCCESS as i32
                }
                Err(e) => e.errno as i32,
            }
        },
    )?;

    // --- IMPLEMENT ALL REMAINING WASI PATH FUNCTIONS ---

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_filestat_get",
        |_caller: Caller<'_, T>,
         fd: i32,
         flags: i32,
         path_ptr: i32,
         path_len: i32,
         filestat_ptr: i32|
         -> i32 {
            log::debug!(
                "path_filestat_get({}, {}, {}, {}, {})",
                fd,
                flags,
                path_ptr,
                path_len,
                filestat_ptr
            );
            let path = unsafe {
                core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                    path_ptr as *const u8,
                    path_len as usize,
                ))
            };
            match filesystem::stat(fd as Fd, flags as u16, path) {
                Ok(stat_val) => {
                    unsafe {
                        let filestat_ptr = filestat_ptr as *mut u64;
                        *filestat_ptr = stat_val;
                    }
                    ERRNO_SUCCESS as i32
                }
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_filestat_set_times",
        |_caller: Caller<'_, T>,
         fd: i32,
         flags: i32,
         path_ptr: i32,
         path_len: i32,
         atim: i64,
         mtim: i64,
         fst_flags: i32|
         -> i32 {
            log::debug!(
                "path_filestat_set_times({}, {}, {}, {}, {}, {}, {})",
                fd,
                flags,
                path_ptr,
                path_len,
                atim,
                mtim,
                fst_flags
            );
            let _path = unsafe {
                core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                    path_ptr as *const u8,
                    path_len as usize,
                ))
            };
            match filesystem::set_times(fd as Fd, atim as u64, mtim as u64) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_link",
        |_caller: Caller<'_, T>,
         old_fd: i32,
         old_flags: i32,
         old_path_ptr: i32,
         old_path_len: i32,
         new_fd: i32,
         new_path_ptr: i32,
         new_path_len: i32|
         -> i32 {
            log::debug!(
                "path_link({}, {}, {}, {}, {}, {}, {})",
                old_fd,
                old_flags,
                old_path_ptr,
                old_path_len,
                new_fd,
                new_path_ptr,
                new_path_len
            );
            let old_path = unsafe {
                core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                    old_path_ptr as *const u8,
                    old_path_len as usize,
                ))
            };
            let new_path = unsafe {
                core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                    new_path_ptr as *const u8,
                    new_path_len as usize,
                ))
            };
            match filesystem::link(
                old_fd as Fd,
                old_flags as u16,
                old_path,
                new_fd as Fd,
                new_path,
            ) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_open",
        |_caller: Caller<'_, T>,
         fd: i32,
         dirflags: i32,
         path_ptr: i32,
         path_len: i32,
         oflags: i32,
         fs_rights_base: i64,
         fs_rights_inheriting: i64,
         fdflags: i32,
         opened_fd_ptr: i32|
         -> i32 {
            log::debug!(
                "path_open({}, {}, {}, {}, {}, {}, {}, {}, {})",
                fd,
                dirflags,
                path_ptr,
                path_len,
                oflags,
                fs_rights_base,
                fs_rights_inheriting,
                fdflags,
                opened_fd_ptr
            );
            let path = unsafe {
                core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                    path_ptr as *const u8,
                    path_len as usize,
                ))
            };
            match filesystem::path_open(
                fd as Fd,
                dirflags as LookupFlags,
                path,
                oflags as OFlags,
                fs_rights_base as Rights,
                fs_rights_inheriting as Rights,
                fdflags as FdFlags,
            ) {
                Ok(opened_fd) => {
                    unsafe {
                        let opened_fd_ptr = opened_fd_ptr as *mut Fd;
                        *opened_fd_ptr = opened_fd;
                    }
                    ERRNO_SUCCESS as i32
                }
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_readlink",
        |_caller: Caller<'_, T>,
         fd: i32,
         path_ptr: i32,
         path_len: i32,
         buf_ptr: i32,
         buf_len: i32,
         nread_ptr: i32|
         -> i32 {
            log::debug!(
                "path_readlink({}, {}, {}, {}, {}, {})",
                fd,
                path_ptr,
                path_len,
                buf_ptr,
                buf_len,
                nread_ptr
            );
            let path = unsafe {
                core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                    path_ptr as *const u8,
                    path_len as usize,
                ))
            };
            match filesystem::readlink_at(fd as Fd, path) {
                Ok(target) => {
                    let bytes = target.as_bytes();
                    let n = bytes.len().min(buf_len as usize);
                    unsafe {
                        core::ptr::copy_nonoverlapping(bytes.as_ptr(), buf_ptr as *mut u8, n);
                        let nread_ptr = nread_ptr as *mut u32;
                        *nread_ptr = n as u32;
                    }
                    ERRNO_SUCCESS as i32
                }
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_remove_directory",
        |_caller: Caller<'_, T>, fd: i32, path_ptr: i32, path_len: i32| -> i32 {
            log::debug!("path_remove_directory({}, {}, {})", fd, path_ptr, path_len);
            let path = unsafe {
                core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                    path_ptr as *const u8,
                    path_len as usize,
                ))
            };
            match filesystem::path_remove_directory(fd as Fd, path) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_rename",
        |_caller: Caller<'_, T>,
         fd: i32,
         old_path_ptr: i32,
         old_path_len: i32,
         new_fd: i32,
         new_path_ptr: i32,
         new_path_len: i32|
         -> i32 {
            log::debug!(
                "path_rename({}, {}, {}, {}, {}, {})",
                fd,
                old_path_ptr,
                old_path_len,
                new_fd,
                new_path_ptr,
                new_path_len
            );
            let old_path = unsafe {
                core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                    old_path_ptr as *const u8,
                    old_path_len as usize,
                ))
            };
            let new_path = unsafe {
                core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                    new_path_ptr as *const u8,
                    new_path_len as usize,
                ))
            };
            match filesystem::rename_at(fd as Fd, old_path, new_fd as Fd, new_path) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_symlink",
        |_caller: Caller<'_, T>,
         old_path_ptr: i32,
         old_path_len: i32,
         fd: i32,
         new_path_ptr: i32,
         new_path_len: i32|
         -> i32 {
            log::debug!(
                "path_symlink({}, {}, {}, {}, {})",
                old_path_ptr,
                old_path_len,
                fd,
                new_path_ptr,
                new_path_len
            );
            let old_path = unsafe {
                core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                    old_path_ptr as *const u8,
                    old_path_len as usize,
                ))
            };
            let new_path = unsafe {
                core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                    new_path_ptr as *const u8,
                    new_path_len as usize,
                ))
            };
            match filesystem::symlink_at(fd as Fd, old_path, new_path) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_unlink_file",
        |_caller: Caller<'_, T>, fd: i32, path_ptr: i32, path_len: i32| -> i32 {
            log::debug!("path_unlink_file({}, {}, {})", fd, path_ptr, path_len);
            let path = unsafe {
                core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                    path_ptr as *const u8,
                    path_len as usize,
                ))
            };
            match filesystem::path_unlink_file(fd as Fd, path) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;
    Ok(())
}
