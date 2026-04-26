// WASI Preview 1 (legacy) implementation for Agave OS
//
// Bridge layer: this is where guest WASM memory is read and written. The
// `filesystem` module below is memory-agnostic — bytes flow guest -> host -> VFS
// here.

use super::types::*;
use super::{cli, filesystem, random};
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use wasmi::{AsContext, Caller, Extern, Linker, Memory, Store};

// Limits to keep guest-controlled lengths from triggering kernel-OOM panics.
// 4 KiB worth of iovec records (512 entries) and a 16 MiB cap on a single read
// or write are well above what real WASI consumers need.
const MAX_IOVS: u32 = 512;
const MAX_IO_BYTES: usize = 16 * 1024 * 1024;

// ---------- guest-memory helpers ----------

fn guest_memory<T>(caller: &mut Caller<'_, T>) -> Option<Memory> {
    match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => Some(mem),
        _ => None,
    }
}

fn read_bytes<T>(
    memory: &Memory,
    caller: &Caller<'_, T>,
    offset: u32,
    len: u32,
) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; len as usize];
    memory
        .read(caller.as_context(), offset as usize, &mut buf)
        .ok()?;
    Some(buf)
}

fn write_bytes<T>(
    memory: &Memory,
    caller: &mut Caller<'_, T>,
    offset: u32,
    data: &[u8],
) -> Result<(), ()> {
    memory.write(caller, offset as usize, data).map_err(|_| ())
}

fn read_string<T>(
    memory: &Memory,
    caller: &Caller<'_, T>,
    ptr: u32,
    len: u32,
) -> Option<String> {
    let bytes = read_bytes(memory, caller, ptr, len)?;
    String::from_utf8(bytes).ok()
}

/// Decode an array of `iovs_len` iovec records (8 bytes each: u32 buf, u32 buf_len)
/// from guest memory. Returns `None` for an out-of-bounds read or for an
/// `iovs_len` above [`MAX_IOVS`].
fn read_iovecs<T>(
    memory: &Memory,
    caller: &Caller<'_, T>,
    iovs_ptr: u32,
    iovs_len: u32,
) -> Option<Vec<IOVec>> {
    if iovs_len > MAX_IOVS {
        return None;
    }
    let bytes = (iovs_len as usize).checked_mul(8)?;
    let mut buf = vec![0u8; bytes];
    memory
        .read(caller.as_context(), iovs_ptr as usize, &mut buf)
        .ok()?;
    let mut out = Vec::with_capacity(iovs_len as usize);
    for chunk in buf.chunks_exact(8) {
        let buf_ptr = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        let buf_len = u32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
        out.push(IOVec {
            buf: buf_ptr,
            buf_len,
        });
    }
    Some(out)
}

/// Total payload size for an iovec array, capped at [`MAX_IO_BYTES`]. Returns
/// `None` if the sum overflows or exceeds the cap.
fn iovec_total_bounded(iovecs: &[IOVec]) -> Option<usize> {
    let mut total: usize = 0;
    for iov in iovecs {
        total = total.checked_add(iov.buf_len as usize)?;
        if total > MAX_IO_BYTES {
            return None;
        }
    }
    Some(total)
}

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
        |mut caller: Caller<'_, T>, environ: i32, environ_buf: i32| -> i32 {
            log::debug!("environ_get({}, {})", environ, environ_buf);
            let envs = ["PATH=/usr/bin", "HOME=/home/user", "USER=agave"];
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            let mut buf_offset: u32 = 0;
            let mut ptr_offset: u32 = 0;
            for env in envs.iter() {
                let bytes = env.as_bytes();
                let mut payload = Vec::with_capacity(bytes.len() + 1);
                payload.extend_from_slice(bytes);
                payload.push(0);
                let env_addr = (environ_buf as u32) + buf_offset;
                if write_bytes(&memory, &mut caller, env_addr, &payload).is_err() {
                    return ERRNO_FAULT as i32;
                }
                let ptr_addr = (environ as u32) + ptr_offset;
                if write_bytes(&memory, &mut caller, ptr_addr, &env_addr.to_le_bytes()).is_err() {
                    return ERRNO_FAULT as i32;
                }
                buf_offset += payload.len() as u32;
                ptr_offset += 4;
            }
            ERRNO_SUCCESS as i32
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "environ_sizes_get",
        |mut caller: Caller<'_, T>, count_ptr: i32, buf_size_ptr: i32| -> i32 {
            log::debug!("environ_sizes_get({}, {})", count_ptr, buf_size_ptr);
            let envs = ["PATH=/usr/bin", "HOME=/home/user", "USER=agave"];
            let count = envs.len() as u32;
            let buf_size: u32 = envs.iter().map(|e| e.len() as u32 + 1).sum();
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            if write_bytes(&memory, &mut caller, count_ptr as u32, &count.to_le_bytes()).is_err() {
                return ERRNO_FAULT as i32;
            }
            if write_bytes(&memory, &mut caller, buf_size_ptr as u32, &buf_size.to_le_bytes())
                .is_err()
            {
                return ERRNO_FAULT as i32;
            }
            ERRNO_SUCCESS as i32
        },
    )?;

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
        |mut caller: Caller<'_, T>, count_ptr: i32, buf_size_ptr: i32| -> i32 {
            log::debug!("args_sizes_get({}, {})", count_ptr, buf_size_ptr);
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            let zero = 0u32.to_le_bytes();
            if write_bytes(&memory, &mut caller, count_ptr as u32, &zero).is_err() {
                return ERRNO_FAULT as i32;
            }
            if write_bytes(&memory, &mut caller, buf_size_ptr as u32, &zero).is_err() {
                return ERRNO_FAULT as i32;
            }
            ERRNO_SUCCESS as i32
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_read",
        |mut caller: Caller<'_, T>, fd: i32, iovs: i32, iovs_len: i32, nread_ptr: i32| -> i32 {
            log::debug!("fd_read({}, {}, {}, {})", fd, iovs, iovs_len, nread_ptr);
            if iovs_len < 0 {
                return ERRNO_INVAL as i32;
            }
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            let iovecs = match read_iovecs(&memory, &caller, iovs as u32, iovs_len as u32) {
                Some(v) => v,
                None => return ERRNO_INVAL as i32,
            };
            let total = match iovec_total_bounded(&iovecs) {
                Some(t) => t,
                None => return ERRNO_INVAL as i32,
            };

            // No bytes requested -> success with nread=0; do not call fd_read
            // (would be a no-op anyway, but skip to avoid an unnecessary lock).
            if total == 0 {
                if write_bytes(&memory, &mut caller, nread_ptr as u32, &0u32.to_le_bytes())
                    .is_err()
                {
                    return ERRNO_FAULT as i32;
                }
                return ERRNO_SUCCESS as i32;
            }

            let data = match filesystem::fd_read(fd as Fd, total) {
                Ok(d) => d,
                Err(e) => return e.errno as i32,
            };

            let mut written: u32 = 0;
            let mut cursor = 0usize;
            for iov in &iovecs {
                if cursor >= data.len() {
                    break;
                }
                let take = (iov.buf_len as usize).min(data.len() - cursor);
                if take == 0 {
                    continue;
                }
                if write_bytes(&memory, &mut caller, iov.buf, &data[cursor..cursor + take])
                    .is_err()
                {
                    return ERRNO_FAULT as i32;
                }
                cursor += take;
                written += take as u32;
            }
            if write_bytes(&memory, &mut caller, nread_ptr as u32, &written.to_le_bytes()).is_err()
            {
                return ERRNO_FAULT as i32;
            }
            ERRNO_SUCCESS as i32
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_write",
        |mut caller: Caller<'_, T>, fd: i32, iovs: i32, iovs_len: i32, nwritten_ptr: i32| -> i32 {
            log::debug!("fd_write({}, {}, {}, {})", fd, iovs, iovs_len, nwritten_ptr);
            if iovs_len < 0 {
                return ERRNO_INVAL as i32;
            }
            // stdout / stderr -> log instead of erroring out, to keep eprintln!
            // and similar working for diagnostics.
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            let iovecs = match read_iovecs(&memory, &caller, iovs as u32, iovs_len as u32) {
                Some(v) => v,
                None => return ERRNO_INVAL as i32,
            };
            let total_bound = match iovec_total_bounded(&iovecs) {
                Some(t) => t,
                None => return ERRNO_INVAL as i32,
            };

            let mut payload: Vec<u8> = Vec::with_capacity(total_bound);
            for iov in &iovecs {
                if iov.buf_len == 0 {
                    continue;
                }
                let bytes = match read_bytes(&memory, &caller, iov.buf, iov.buf_len) {
                    Some(b) => b,
                    None => return ERRNO_FAULT as i32,
                };
                payload.extend_from_slice(&bytes);
            }

            let total = payload.len() as u32;

            if fd == 1 || fd == 2 {
                if let Ok(s) = core::str::from_utf8(&payload) {
                    if fd == 1 {
                        log::info!("[wasi stdout] {}", s.trim_end());
                    } else {
                        log::warn!("[wasi stderr] {}", s.trim_end());
                    }
                }
                if write_bytes(&memory, &mut caller, nwritten_ptr as u32, &total.to_le_bytes())
                    .is_err()
                {
                    return ERRNO_FAULT as i32;
                }
                return ERRNO_SUCCESS as i32;
            }

            match filesystem::fd_write(fd as Fd, &payload) {
                Ok(n) => {
                    if write_bytes(&memory, &mut caller, nwritten_ptr as u32, &n.to_le_bytes())
                        .is_err()
                    {
                        return ERRNO_FAULT as i32;
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
        |mut caller: Caller<'_, T>, buf: i32, buf_len: i32| -> i32 {
            log::debug!("random_get({}, {})", buf, buf_len);
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            match random::get_random_bytes(buf_len as u64) {
                Ok(data) => {
                    if write_bytes(&memory, &mut caller, buf as u32, &data).is_err() {
                        return ERRNO_FAULT as i32;
                    }
                    ERRNO_SUCCESS as i32
                }
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
        "fd_readdir",
        |mut caller: Caller<'_, T>,
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
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            let entries = match filesystem::fd_readdir_entries(fd as Fd, cookie as DirCookie) {
                Ok(e) => e,
                Err(e) => return e.errno as i32,
            };

            // Serialise into the Preview 1 dirent ABI:
            //   d_next:  u64 (next cookie = absolute index + 1)
            //   d_ino:   u64
            //   d_namlen:u32
            //   d_type:  u8
            //   3 bytes pad (implicit, so the header is 24 bytes total)
            //   name bytes (no NUL)
            //
            // Stop adding records once the next one would overflow `buf_len`.
            // WASI requires truncation at entry boundaries — a dirent that's
            // cut mid-record corrupts the d_next cookie the guest reads to
            // resume, silently breaking iteration.
            if buf_len < 0 {
                return ERRNO_INVAL as i32;
            }
            let cap = buf_len as usize;
            let mut out: Vec<u8> = Vec::new();
            for (i, entry) in entries.iter().enumerate() {
                let record_size = 24 + entry.name.len();
                if out.len() + record_size > cap {
                    break;
                }
                let next_cookie = (cookie as u64) + (i as u64) + 1;
                let mut header = [0u8; 24];
                header[0..8].copy_from_slice(&next_cookie.to_le_bytes());
                header[8..16].copy_from_slice(&entry.ino.to_le_bytes());
                header[16..20].copy_from_slice(&(entry.name.len() as u32).to_le_bytes());
                header[20] = entry.file_type;
                out.extend_from_slice(&header);
                out.extend_from_slice(entry.name.as_bytes());
            }

            let take = out.len();
            if take > 0
                && write_bytes(&memory, &mut caller, buf as u32, &out[..take]).is_err()
            {
                return ERRNO_FAULT as i32;
            }
            if write_bytes(
                &memory,
                &mut caller,
                bufused_ptr as u32,
                &(take as u32).to_le_bytes(),
            )
            .is_err()
            {
                return ERRNO_FAULT as i32;
            }
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

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_fdstat_get",
        |mut caller: Caller<'_, T>, fd: i32, stat_ptr: i32| -> i32 {
            log::debug!("fd_fdstat_get({}, {})", fd, stat_ptr);
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            // stdio fds: report a TTY-ish character device.
            if fd == 0 || fd == 1 || fd == 2 {
                let mut fdstat = [0u8; 24];
                fdstat[0] = FILETYPE_CHARACTER_DEVICE;
                let rights = if fd == 0 {
                    RIGHTS_FD_READ
                } else {
                    RIGHTS_FD_WRITE
                };
                fdstat[8..16].copy_from_slice(&rights.to_le_bytes());
                fdstat[16..24].copy_from_slice(&rights.to_le_bytes());
                if write_bytes(&memory, &mut caller, stat_ptr as u32, &fdstat).is_err() {
                    return ERRNO_FAULT as i32;
                }
                return ERRNO_SUCCESS as i32;
            }
            match filesystem::fd_fdstat_get(fd as Fd) {
                Ok(fdstat) => {
                    if write_bytes(&memory, &mut caller, stat_ptr as u32, &fdstat).is_err() {
                        return ERRNO_FAULT as i32;
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
        |mut caller: Caller<'_, T>, fd: i32, prestat_ptr: i32| -> i32 {
            log::debug!("fd_prestat_get({}, {})", fd, prestat_ptr);
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            match filesystem::fd_prestat_get(fd as Fd) {
                Ok(prestat) => {
                    // The prestat layout is: u8 tag, 3 bytes pad, u32 pr_name_len.
                    let mut buf = [0u8; 8];
                    buf[0] = prestat.tag;
                    let len = unsafe { prestat.u.dir.pr_name_len };
                    buf[4..8].copy_from_slice(&len.to_le_bytes());
                    if write_bytes(&memory, &mut caller, prestat_ptr as u32, &buf).is_err() {
                        return ERRNO_FAULT as i32;
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
        |mut caller: Caller<'_, T>, fd: i32, path: i32, path_len: i32| -> i32 {
            log::debug!("fd_prestat_dir_name({}, {}, {})", fd, path, path_len);
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            match filesystem::fd_prestat_dir_name(fd as Fd) {
                Ok(name) => {
                    let bytes = name.as_bytes();
                    if bytes.len() > path_len as usize {
                        return ERRNO_NAMETOOLONG as i32;
                    }
                    if write_bytes(&memory, &mut caller, path as u32, bytes).is_err() {
                        return ERRNO_FAULT as i32;
                    }
                    ERRNO_SUCCESS as i32
                }
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "sched_yield",
        |_caller: Caller<'_, T>| -> i32 { ERRNO_SUCCESS as i32 },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_filestat_get",
        |mut caller: Caller<'_, T>, fd: i32, filestat_ptr: i32| -> i32 {
            log::debug!("fd_filestat_get({}, {})", fd, filestat_ptr);
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            match filesystem::fd_filestat_get(fd as Fd) {
                Ok(filestat) => {
                    if write_bytes(&memory, &mut caller, filestat_ptr as u32, &filestat).is_err()
                    {
                        return ERRNO_FAULT as i32;
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
        |mut caller: Caller<'_, T>, fd: i32, offset_ptr: i32| -> i32 {
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            match filesystem::fd_tell(fd as Fd) {
                Ok(offset) => {
                    if write_bytes(&memory, &mut caller, offset_ptr as u32, &offset.to_le_bytes())
                        .is_err()
                    {
                        return ERRNO_FAULT as i32;
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
        |mut caller: Caller<'_, T>, fd: i32, offset: i64, whence: i32, new_offset_ptr: i32| -> i32 {
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            match filesystem::fd_seek(fd as Fd, offset as FileDelta, whence as Whence) {
                Ok(new_offset) => {
                    if write_bytes(
                        &memory,
                        &mut caller,
                        new_offset_ptr as u32,
                        &new_offset.to_le_bytes(),
                    )
                    .is_err()
                    {
                        return ERRNO_FAULT as i32;
                    }
                    ERRNO_SUCCESS as i32
                }
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_create_directory",
        |mut caller: Caller<'_, T>, fd: i32, path_ptr: i32, path_len: i32| -> i32 {
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            let path = match read_string(&memory, &caller, path_ptr as u32, path_len as u32) {
                Some(p) => p,
                None => return ERRNO_FAULT as i32,
            };
            match filesystem::path_create_directory(fd as Fd, &path) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_filestat_get",
        |mut caller: Caller<'_, T>,
         fd: i32,
         flags: i32,
         path_ptr: i32,
         path_len: i32,
         filestat_ptr: i32|
         -> i32 {
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            let path = match read_string(&memory, &caller, path_ptr as u32, path_len as u32) {
                Some(p) => p,
                None => return ERRNO_FAULT as i32,
            };
            // Resolve through filesystem::stat so the path is confined to
            // the parent fd's preopen sandbox (no `..` escape).
            let resolved = match filesystem::stat_resolve(fd as Fd, flags as u16, &path) {
                Ok(p) => p,
                Err(e) => return e.errno as i32,
            };
            let metadata = match crate::sys::fs::metadata(&resolved) {
                Ok(m) => m,
                Err(_) => return ERRNO_NOENT as i32,
            };
            let file_type = match metadata.file_type {
                crate::sys::fs::FileType::Regular => FILETYPE_REGULAR_FILE,
                crate::sys::fs::FileType::Directory => FILETYPE_DIRECTORY,
                crate::sys::fs::FileType::Symlink => FILETYPE_SYMBOLIC_LINK,
                _ => FILETYPE_UNKNOWN,
            };
            let mut filestat = [0u8; 56];
            filestat[0..8].copy_from_slice(&1u64.to_le_bytes()); // dev
            filestat[8..16].copy_from_slice(&(fd as u64).to_le_bytes()); // ino
            filestat[16] = file_type;
            filestat[24..32].copy_from_slice(&1u64.to_le_bytes()); // nlink
            filestat[32..40].copy_from_slice(&metadata.size.to_le_bytes());
            filestat[40..48].copy_from_slice(&metadata.modified_time.to_le_bytes());
            filestat[48..56].copy_from_slice(&metadata.modified_time.to_le_bytes());
            if write_bytes(&memory, &mut caller, filestat_ptr as u32, &filestat).is_err() {
                return ERRNO_FAULT as i32;
            }
            ERRNO_SUCCESS as i32
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_filestat_set_times",
        |mut caller: Caller<'_, T>,
         fd: i32,
         _flags: i32,
         path_ptr: i32,
         path_len: i32,
         atim: i64,
         mtim: i64,
         _fst_flags: i32|
         -> i32 {
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            let _ = match read_string(&memory, &caller, path_ptr as u32, path_len as u32) {
                Some(p) => p,
                None => return ERRNO_FAULT as i32,
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
        |mut caller: Caller<'_, T>,
         old_fd: i32,
         old_flags: i32,
         old_path_ptr: i32,
         old_path_len: i32,
         new_fd: i32,
         new_path_ptr: i32,
         new_path_len: i32|
         -> i32 {
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            let old_path =
                match read_string(&memory, &caller, old_path_ptr as u32, old_path_len as u32) {
                    Some(p) => p,
                    None => return ERRNO_FAULT as i32,
                };
            let new_path =
                match read_string(&memory, &caller, new_path_ptr as u32, new_path_len as u32) {
                    Some(p) => p,
                    None => return ERRNO_FAULT as i32,
                };
            match filesystem::link(
                old_fd as Fd,
                old_flags as u16,
                &old_path,
                new_fd as Fd,
                &new_path,
            ) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_open",
        |mut caller: Caller<'_, T>,
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
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            let path = match read_string(&memory, &caller, path_ptr as u32, path_len as u32) {
                Some(p) => p,
                None => return ERRNO_FAULT as i32,
            };
            log::debug!("path_open(fd={}, path={:?}, oflags={})", fd, path, oflags);
            match filesystem::path_open(
                fd as Fd,
                dirflags as LookupFlags,
                &path,
                oflags as OFlags,
                fs_rights_base as Rights,
                fs_rights_inheriting as Rights,
                fdflags as FdFlags,
            ) {
                Ok(opened_fd) => {
                    if write_bytes(
                        &memory,
                        &mut caller,
                        opened_fd_ptr as u32,
                        &opened_fd.to_le_bytes(),
                    )
                    .is_err()
                    {
                        return ERRNO_FAULT as i32;
                    }
                    ERRNO_SUCCESS as i32
                }
                Err(e) => {
                    log::debug!("path_open error: errno={}", e.errno);
                    e.errno as i32
                }
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_readlink",
        |mut caller: Caller<'_, T>,
         fd: i32,
         path_ptr: i32,
         path_len: i32,
         buf_ptr: i32,
         buf_len: i32,
         nread_ptr: i32|
         -> i32 {
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            let path = match read_string(&memory, &caller, path_ptr as u32, path_len as u32) {
                Some(p) => p,
                None => return ERRNO_FAULT as i32,
            };
            match filesystem::readlink_at(fd as Fd, &path) {
                Ok(target) => {
                    let bytes = target.as_bytes();
                    let n = bytes.len().min(buf_len as usize) as u32;
                    if write_bytes(&memory, &mut caller, buf_ptr as u32, &bytes[..n as usize])
                        .is_err()
                    {
                        return ERRNO_FAULT as i32;
                    }
                    if write_bytes(&memory, &mut caller, nread_ptr as u32, &n.to_le_bytes())
                        .is_err()
                    {
                        return ERRNO_FAULT as i32;
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
        |mut caller: Caller<'_, T>, fd: i32, path_ptr: i32, path_len: i32| -> i32 {
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            let path = match read_string(&memory, &caller, path_ptr as u32, path_len as u32) {
                Some(p) => p,
                None => return ERRNO_FAULT as i32,
            };
            match filesystem::path_remove_directory(fd as Fd, &path) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_rename",
        |mut caller: Caller<'_, T>,
         fd: i32,
         old_path_ptr: i32,
         old_path_len: i32,
         new_fd: i32,
         new_path_ptr: i32,
         new_path_len: i32|
         -> i32 {
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            let old_path =
                match read_string(&memory, &caller, old_path_ptr as u32, old_path_len as u32) {
                    Some(p) => p,
                    None => return ERRNO_FAULT as i32,
                };
            let new_path =
                match read_string(&memory, &caller, new_path_ptr as u32, new_path_len as u32) {
                    Some(p) => p,
                    None => return ERRNO_FAULT as i32,
                };
            match filesystem::rename_at(fd as Fd, &old_path, new_fd as Fd, &new_path) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_symlink",
        |mut caller: Caller<'_, T>,
         old_path_ptr: i32,
         old_path_len: i32,
         fd: i32,
         new_path_ptr: i32,
         new_path_len: i32|
         -> i32 {
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            let old_path =
                match read_string(&memory, &caller, old_path_ptr as u32, old_path_len as u32) {
                    Some(p) => p,
                    None => return ERRNO_FAULT as i32,
                };
            let new_path =
                match read_string(&memory, &caller, new_path_ptr as u32, new_path_len as u32) {
                    Some(p) => p,
                    None => return ERRNO_FAULT as i32,
                };
            match filesystem::symlink_at(fd as Fd, &old_path, &new_path) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "path_unlink_file",
        |mut caller: Caller<'_, T>, fd: i32, path_ptr: i32, path_len: i32| -> i32 {
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            let path = match read_string(&memory, &caller, path_ptr as u32, path_len as u32) {
                Some(p) => p,
                None => return ERRNO_FAULT as i32,
            };
            match filesystem::path_unlink_file(fd as Fd, &path) {
                Ok(()) => ERRNO_SUCCESS as i32,
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "clock_time_get",
        |mut caller: Caller<'_, T>, clock_id: i32, precision: i64, time_ptr: i32| -> i32 {
            let memory = match guest_memory(&mut caller) {
                Some(m) => m,
                None => return ERRNO_FAULT as i32,
            };
            match super::clocks::clock_time_get(clock_id as Clockid, precision as Timestamp) {
                Ok(t) => {
                    if write_bytes(&memory, &mut caller, time_ptr as u32, &t.to_le_bytes())
                        .is_err()
                    {
                        return ERRNO_FAULT as i32;
                    }
                    ERRNO_SUCCESS as i32
                }
                Err(e) => e.errno as i32,
            }
        },
    )?;

    linker.func_wrap(
        "wasi_snapshot_preview1",
        "poll_oneoff",
        |_caller: Caller<'_, T>,
         _in_ptr: i32,
         _out_ptr: i32,
         _nsubscriptions: i32,
         _nevents_ptr: i32|
         -> i32 {
            ERRNO_SUCCESS as i32
        },
    )?;

    Ok(())
}
