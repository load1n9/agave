//! WASI filesystem layer.
//!
//! This is the logical layer of the WASI filesystem implementation. It owns
//! the per-fd table (offsets, rights, paths) and translates WASI semantics
//! onto the kernel's global `VirtualFileSystem` (`crate::sys::fs`). It does
//! **not** touch WASM linear memory — that is the bridge layer's job
//! (`super::preview1`).
//!
//! Read/write/readdir signatures here are deliberately memory-agnostic: they
//! accept and return plain Rust slices and `Vec<u8>`. The bridge does the
//! guest-memory copy.

use super::error::*;
use super::types::*;
use crate::sys::fs;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use spin::Mutex;

// Per-fd table. Tracks what each open WASI fd points at and where it is in the
// file. The actual file bytes live in the kernel VFS, not here.
//
// Named FD_TABLE rather than FILESYSTEM to avoid grep-confusion with the
// kernel's `static mut FILESYSTEM` in `crate::sys::fs`.
static FD_TABLE: Mutex<FilesystemState> = Mutex::new(FilesystemState::new());

/// Cap the number of simultaneously-open WASI fds. Prevents a runaway guest
/// from exhausting kernel heap by calling `path_open` in a loop.
const MAX_OPEN_FDS: usize = 256;

#[derive(Debug)]
pub struct FilesystemState {
    open_files: BTreeMap<Fd, FileDescriptor>,
    preopened_dirs: BTreeMap<Fd, String>,
    next_fd: Fd,
}

impl FilesystemState {
    pub const fn new() -> Self {
        Self {
            open_files: BTreeMap::new(),
            preopened_dirs: BTreeMap::new(),
            next_fd: 3, // Start after stdin(0), stdout(1), stderr(2)
        }
    }

    pub fn allocate_fd(&mut self) -> Fd {
        let fd = self.next_fd;
        self.next_fd += 1;
        fd
    }

    pub fn add_preopen(&mut self, path: String) -> Fd {
        let fd = self.allocate_fd();
        self.preopened_dirs.insert(fd, path);
        fd
    }
}

#[derive(Debug, Clone)]
pub struct FileDescriptor {
    pub path: String,
    pub flags: FdFlags,
    pub rights_base: Rights,
    pub rights_inheriting: Rights,
    pub file_type: u8,
    pub offset: FileSize,
    pub size: FileSize,
    pub is_directory: bool,
}

impl FileDescriptor {
    pub fn new(
        path: String,
        flags: FdFlags,
        rights_base: Rights,
        rights_inheriting: Rights,
    ) -> Self {
        Self {
            path,
            flags,
            rights_base,
            rights_inheriting,
            file_type: FILETYPE_REGULAR_FILE,
            offset: 0,
            size: 0,
            is_directory: false,
        }
    }

    pub fn new_directory(path: String, rights_base: Rights, rights_inheriting: Rights) -> Self {
        Self {
            path,
            flags: 0,
            rights_base,
            rights_inheriting,
            file_type: FILETYPE_DIRECTORY,
            offset: 0,
            size: 0,
            is_directory: true,
        }
    }
}

/// Initialise WASI preopens. Idempotent. The kernel's `VirtualFileSystem` is
/// initialised separately by `crate::sys::fs::init_filesystem_with_type`; this
/// function only sets up the WASI fd table so guests can resolve `/` and
/// `/tmp` via `fd_prestat_get`.
pub fn init_filesystem() {
    let mut fs_state = FD_TABLE.lock();
    // Idempotent: if preopens already set, skip.
    if !fs_state.preopened_dirs.is_empty() {
        return;
    }
    fs_state.add_preopen("/".to_string());
    fs_state.add_preopen("/tmp".to_string());
}

// ---------- Preview 1 implementations ----------

pub fn fd_prestat_get(fd: Fd) -> WasiResult<Prestat> {
    let fs_state = FD_TABLE.lock();

    if let Some(path) = fs_state.preopened_dirs.get(&fd) {
        Ok(Prestat {
            tag: 0, // PREOPENTYPE_DIR
            u: PrestatU {
                dir: PrestatDir {
                    pr_name_len: path.len() as Size,
                },
            },
        })
    } else {
        Err(WasiError::badf())
    }
}

/// Returns the preopen path string. The bridge writes it to guest memory.
pub fn fd_prestat_dir_name(fd: Fd) -> WasiResult<String> {
    let fs_state = FD_TABLE.lock();
    fs_state
        .preopened_dirs
        .get(&fd)
        .cloned()
        .ok_or_else(WasiError::badf)
}

pub fn path_open(
    fd: Fd,
    _dirflags: LookupFlags,
    path: &str,
    oflags: OFlags,
    fs_rights_base: Rights,
    fs_rights_inheriting: Rights,
    fdflags: FdFlags,
) -> WasiResult<Fd> {
    let mut fs_state = FD_TABLE.lock();

    if fs_state.open_files.len() >= MAX_OPEN_FDS {
        return Err(WasiError::new(ERRNO_NFILE, "Too many open files"));
    }

    // Locate the base path and parent rights for `fd` (preopen or open dir).
    // Preopens get the full directory rights set; child fds inherit the parent's
    // `rights_inheriting`. Both branches return `(base_path, max_rights)`.
    let (base_path, parent_rights) =
        if let Some(preopen_path) = fs_state.preopened_dirs.get(&fd) {
            let max_rights = RIGHTS_FD_READ
                | RIGHTS_FD_WRITE
                | RIGHTS_FD_SEEK
                | RIGHTS_FD_TELL
                | RIGHTS_FD_FDSTAT_SET_FLAGS
                | RIGHTS_FD_SYNC
                | RIGHTS_FD_DATASYNC
                | RIGHTS_FD_ADVISE
                | RIGHTS_FD_ALLOCATE
                | RIGHTS_FD_FILESTAT_GET
                | RIGHTS_FD_FILESTAT_SET_SIZE
                | RIGHTS_FD_FILESTAT_SET_TIMES
                | RIGHTS_FD_READDIR
                | RIGHTS_PATH_OPEN
                | RIGHTS_PATH_CREATE_DIRECTORY
                | RIGHTS_PATH_CREATE_FILE
                | RIGHTS_PATH_FILESTAT_GET
                | RIGHTS_PATH_FILESTAT_SET_SIZE
                | RIGHTS_PATH_FILESTAT_SET_TIMES
                | RIGHTS_PATH_LINK_SOURCE
                | RIGHTS_PATH_LINK_TARGET
                | RIGHTS_PATH_RENAME_SOURCE
                | RIGHTS_PATH_RENAME_TARGET
                | RIGHTS_PATH_READLINK
                | RIGHTS_PATH_REMOVE_DIRECTORY
                | RIGHTS_PATH_UNLINK_FILE
                | RIGHTS_PATH_SYMLINK;
            (preopen_path.clone(), max_rights)
        } else if let Some(file_desc) = fs_state.open_files.get(&fd) {
            if !file_desc.is_directory {
                return Err(WasiError::notdir());
            }
            (file_desc.path.clone(), file_desc.rights_inheriting)
        } else {
            return Err(WasiError::badf());
        };

    // Intersect guest-requested rights with what the parent allows. WASI requires
    // hosts to refuse to grant rights the parent fd does not itself inherit.
    let granted_base = fs_rights_base & parent_rights;
    let granted_inheriting = fs_rights_inheriting & parent_rights;

    // Resolve `..` / `.` / empty segments and confine to the preopen root.
    let full_path = match resolve_under(&base_path, path) {
        Some(p) => p,
        None => return Err(WasiError::notcapable()),
    };

    let exists = fs::exists(&full_path);
    let is_dir = exists && fs::is_dir(&full_path);

    // WASI OFLAGS bits: CREAT=0x1, DIRECTORY=0x2, EXCL=0x4, TRUNC=0x8.
    let o_creat = (oflags & 0x1) != 0;
    let o_excl = (oflags & 0x4) != 0;

    if exists && o_creat && o_excl {
        return Err(WasiError::exist());
    }
    if !exists {
        if !o_creat {
            return Err(WasiError::noent());
        }
        fs::write_file(&full_path, Vec::new()).map_err(|_| WasiError::io())?;
    }

    let new_fd = fs_state.allocate_fd();
    let mut file_desc = if is_dir {
        FileDescriptor::new_directory(full_path.clone(), granted_base, granted_inheriting)
    } else {
        FileDescriptor::new(full_path.clone(), fdflags, granted_base, granted_inheriting)
    };
    if !file_desc.is_directory {
        file_desc.size = fs::metadata(&full_path)
            .map(|m| m.size)
            .unwrap_or(0);
    }
    fs_state.open_files.insert(new_fd, file_desc);
    Ok(new_fd)
}

/// Read up to `total_len` bytes starting at the fd's current offset. Returns
/// the bytes read; advances the fd offset by the number of bytes returned.
pub fn fd_read(fd: Fd, total_len: usize) -> WasiResult<Vec<u8>> {
    let mut fs_state = FD_TABLE.lock();

    let file_desc = fs_state
        .open_files
        .get_mut(&fd)
        .ok_or_else(WasiError::badf)?;

    if (file_desc.rights_base & RIGHTS_FD_READ) == 0 {
        return Err(WasiError::notcapable());
    }
    if file_desc.is_directory {
        return Err(WasiError::isdir());
    }

    let file_data = fs::read_file(&file_desc.path).map_err(|_| WasiError::io())?;

    let start = file_desc.offset as usize;
    if start >= file_data.len() {
        return Ok(Vec::new());
    }
    let end = (start + total_len).min(file_data.len());
    let bytes = file_data[start..end].to_vec();
    file_desc.offset += bytes.len() as FileSize;
    Ok(bytes)
}

/// Write `data` at the fd's current offset, extending the file if needed.
/// Returns the number of bytes written; advances the fd offset.
pub fn fd_write(fd: Fd, data: &[u8]) -> WasiResult<Size> {
    let mut fs_state = FD_TABLE.lock();

    let file_desc = fs_state
        .open_files
        .get_mut(&fd)
        .ok_or_else(WasiError::badf)?;

    if (file_desc.rights_base & RIGHTS_FD_WRITE) == 0 {
        return Err(WasiError::notcapable());
    }
    if file_desc.is_directory {
        return Err(WasiError::isdir());
    }
    if data.is_empty() {
        return Ok(0);
    }

    let mut file_data = fs::read_file(&file_desc.path).unwrap_or_default();
    let offset = file_desc.offset as usize;
    let end = offset + data.len();
    if end > file_data.len() {
        file_data.resize(end, 0);
    }
    file_data[offset..end].copy_from_slice(data);
    fs::write_file(&file_desc.path, file_data).map_err(|_| WasiError::io())?;

    file_desc.offset = end as FileSize;
    file_desc.size = file_desc.size.max(end as FileSize);
    Ok(data.len() as Size)
}

pub fn fd_seek(fd: Fd, offset: FileDelta, whence: Whence) -> WasiResult<FileSize> {
    let mut fs_state = FD_TABLE.lock();

    let file_desc = fs_state
        .open_files
        .get_mut(&fd)
        .ok_or_else(WasiError::badf)?;
    if (file_desc.rights_base & RIGHTS_FD_SEEK) == 0 {
        return Err(WasiError::notcapable());
    }

    let new_offset = match whence {
        0 => {
            // SEEK_SET requires a non-negative absolute offset.
            if offset < 0 {
                return Err(WasiError::inval());
            }
            offset as FileSize
        }
        1 => file_desc.offset.saturating_add_signed(offset), // SEEK_CUR
        2 => file_desc.size.saturating_add_signed(offset),   // SEEK_END
        _ => return Err(WasiError::inval()),
    };

    file_desc.offset = new_offset;
    Ok(new_offset)
}

pub fn fd_tell(fd: Fd) -> WasiResult<FileSize> {
    let fs_state = FD_TABLE.lock();

    let file_desc = fs_state
        .open_files
        .get(&fd)
        .ok_or_else(WasiError::badf)?;
    if (file_desc.rights_base & RIGHTS_FD_TELL) == 0 {
        return Err(WasiError::notcapable());
    }
    Ok(file_desc.offset)
}

pub fn fd_close(fd: Fd) -> WasiResult<()> {
    let mut fs_state = FD_TABLE.lock();
    fs_state
        .open_files
        .remove(&fd)
        .ok_or_else(WasiError::badf)?;
    Ok(())
}

pub fn fd_sync(fd: Fd) -> WasiResult<()> {
    let fs_state = FD_TABLE.lock();
    let file_desc = fs_state
        .open_files
        .get(&fd)
        .ok_or_else(WasiError::badf)?;
    if (file_desc.rights_base & RIGHTS_FD_SYNC) == 0 {
        return Err(WasiError::notcapable());
    }
    // Kernel VFS write-through: writes are already persisted. Sync is a no-op.
    Ok(())
}

pub fn fd_datasync(fd: Fd) -> WasiResult<()> {
    let fs_state = FD_TABLE.lock();
    let file_desc = fs_state
        .open_files
        .get(&fd)
        .ok_or_else(WasiError::badf)?;
    if (file_desc.rights_base & RIGHTS_FD_DATASYNC) == 0 {
        return Err(WasiError::notcapable());
    }
    Ok(())
}

pub fn fd_allocate(fd: Fd, offset: FileSize, len: FileSize) -> WasiResult<()> {
    let mut fs_state = FD_TABLE.lock();
    let file_desc = fs_state
        .open_files
        .get_mut(&fd)
        .ok_or_else(WasiError::badf)?;
    if (file_desc.rights_base & RIGHTS_FD_ALLOCATE) == 0 {
        return Err(WasiError::notcapable());
    }

    // Refuse silently-wrapping arithmetic; refuse sizes that would not fit on
    // a 64-bit host's heap.
    let new_size = offset.checked_add(len).ok_or_else(WasiError::fbig)?;
    if new_size > usize::MAX as FileSize {
        return Err(WasiError::fbig());
    }
    if new_size > file_desc.size {
        let path = file_desc.path.clone();
        let mut data = fs::read_file(&path).unwrap_or_default();
        data.resize(new_size as usize, 0);
        fs::write_file(&path, data).map_err(|_| WasiError::io())?;
        file_desc.size = new_size;
    }
    Ok(())
}

pub fn fd_advise(fd: Fd, _offset: FileSize, _len: FileSize, _advice: Advice) -> WasiResult<()> {
    let fs_state = FD_TABLE.lock();
    let file_desc = fs_state
        .open_files
        .get(&fd)
        .ok_or_else(WasiError::badf)?;
    if (file_desc.rights_base & RIGHTS_FD_ADVISE) == 0 {
        return Err(WasiError::notcapable());
    }
    Ok(())
}

pub fn fd_fdstat_get(fd: Fd) -> WasiResult<FdStat> {
    let fs_state = FD_TABLE.lock();

    if let Some(file_desc) = fs_state.open_files.get(&fd) {
        let mut fdstat = [0u8; 24];
        fdstat[0] = file_desc.file_type;
        fdstat[2..4].copy_from_slice(&file_desc.flags.to_le_bytes());
        fdstat[8..16].copy_from_slice(&file_desc.rights_base.to_le_bytes());
        fdstat[16..24].copy_from_slice(&file_desc.rights_inheriting.to_le_bytes());
        Ok(fdstat)
    } else if fs_state.preopened_dirs.contains_key(&fd) {
        let mut fdstat = [0u8; 24];
        fdstat[0] = FILETYPE_DIRECTORY;
        let rights = RIGHTS_FD_READ
            | RIGHTS_PATH_OPEN
            | RIGHTS_FD_READDIR
            | RIGHTS_PATH_CREATE_DIRECTORY
            | RIGHTS_PATH_CREATE_FILE
            | RIGHTS_PATH_FILESTAT_GET
            | RIGHTS_PATH_UNLINK_FILE
            | RIGHTS_PATH_REMOVE_DIRECTORY;
        fdstat[8..16].copy_from_slice(&rights.to_le_bytes());
        fdstat[16..24].copy_from_slice(&rights.to_le_bytes());
        Ok(fdstat)
    } else {
        Err(WasiError::badf())
    }
}

pub fn fd_fdstat_set_flags(fd: Fd, flags: FdFlags) -> WasiResult<()> {
    let mut fs_state = FD_TABLE.lock();
    let file_desc = fs_state
        .open_files
        .get_mut(&fd)
        .ok_or_else(WasiError::badf)?;
    if (file_desc.rights_base & RIGHTS_FD_FDSTAT_SET_FLAGS) == 0 {
        return Err(WasiError::notcapable());
    }
    file_desc.flags = flags;
    Ok(())
}

pub fn fd_filestat_get(fd: Fd) -> WasiResult<FileStat> {
    let fs_state = FD_TABLE.lock();

    let file_desc = fs_state
        .open_files
        .get(&fd)
        .ok_or_else(WasiError::badf)?;
    if (file_desc.rights_base & RIGHTS_FD_FILESTAT_GET) == 0 {
        return Err(WasiError::notcapable());
    }

    let mut filestat = [0u8; 56];
    filestat[0..8].copy_from_slice(&1u64.to_le_bytes());
    filestat[8..16].copy_from_slice(&(fd as u64).to_le_bytes());
    filestat[16] = file_desc.file_type;
    filestat[24..32].copy_from_slice(&1u64.to_le_bytes());
    filestat[32..40].copy_from_slice(&file_desc.size.to_le_bytes());
    let current_time = super::clocks::clock_time_get(CLOCKID_REALTIME, 0).unwrap_or(0);
    filestat[40..48].copy_from_slice(&current_time.to_le_bytes());
    filestat[48..56].copy_from_slice(&current_time.to_le_bytes());
    Ok(filestat)
}

pub fn fd_filestat_set_size(fd: Fd, size: FileSize) -> WasiResult<()> {
    let mut fs_state = FD_TABLE.lock();
    let file_desc = fs_state
        .open_files
        .get_mut(&fd)
        .ok_or_else(WasiError::badf)?;
    if (file_desc.rights_base & RIGHTS_FD_FILESTAT_SET_SIZE) == 0 {
        return Err(WasiError::notcapable());
    }

    if file_desc.is_directory {
        return Err(WasiError::isdir());
    }

    if size > usize::MAX as FileSize {
        return Err(WasiError::fbig());
    }

    let path = file_desc.path.clone();
    let mut data = fs::read_file(&path).unwrap_or_default();
    data.resize(size as usize, 0);
    fs::write_file(&path, data).map_err(|_| WasiError::io())?;
    file_desc.size = size;
    if file_desc.offset > size {
        file_desc.offset = size;
    }
    Ok(())
}

pub fn path_create_directory(fd: Fd, path: &str) -> WasiResult<()> {
    let fs_state = FD_TABLE.lock();
    let base_path = base_path_of(&fs_state, fd)?;
    drop(fs_state);
    let full_path = resolve_under(&base_path, path).ok_or_else(WasiError::notcapable)?;
    fs::create_dir_all(&full_path).map_err(|_| WasiError::io())
}

pub fn path_unlink_file(fd: Fd, path: &str) -> WasiResult<()> {
    let fs_state = FD_TABLE.lock();
    let base_path = base_path_of(&fs_state, fd)?;
    drop(fs_state);
    let full_path = resolve_under(&base_path, path).ok_or_else(WasiError::notcapable)?;
    if !fs::exists(&full_path) {
        return Err(WasiError::noent());
    }
    if fs::is_dir(&full_path) {
        return Err(WasiError::isdir());
    }
    fs::remove(&full_path).map_err(|_| WasiError::io())
}

pub fn path_remove_directory(fd: Fd, path: &str) -> WasiResult<()> {
    let fs_state = FD_TABLE.lock();
    let base_path = base_path_of(&fs_state, fd)?;
    drop(fs_state);
    let full_path = resolve_under(&base_path, path).ok_or_else(WasiError::notcapable)?;
    if !fs::exists(&full_path) {
        return Err(WasiError::noent());
    }
    if !fs::is_dir(&full_path) {
        return Err(WasiError::notdir());
    }
    fs::remove(&full_path).map_err(|_| WasiError::io())
}

/// Directory entry as returned to the bridge, which then serialises into the
/// WASI dirent ABI.
#[derive(Debug, Clone)]
pub struct DirEntryRecord {
    pub ino: u64,
    pub name: String,
    pub file_type: u8,
}

/// List directory entries starting at `cookie` (entry index). The bridge
/// serialises and writes to guest memory.
pub fn fd_readdir_entries(fd: Fd, cookie: DirCookie) -> WasiResult<Vec<DirEntryRecord>> {
    let fs_state = FD_TABLE.lock();
    let path = if let Some(file_desc) = fs_state.open_files.get(&fd) {
        if !file_desc.is_directory || (file_desc.rights_base & RIGHTS_FD_READDIR) == 0 {
            return Err(WasiError::notdir());
        }
        file_desc.path.clone()
    } else if let Some(path) = fs_state.preopened_dirs.get(&fd) {
        path.clone()
    } else {
        return Err(WasiError::badf());
    };
    drop(fs_state);

    let entries = fs::read_dir(&path).map_err(|_| WasiError::io())?;
    let mut records = Vec::with_capacity(entries.len());
    for (idx, entry) in entries.into_iter().enumerate().skip(cookie as usize) {
        let file_type = match entry.file_type {
            crate::sys::fs::FileType::Regular => FILETYPE_REGULAR_FILE,
            crate::sys::fs::FileType::Directory => FILETYPE_DIRECTORY,
            crate::sys::fs::FileType::Symlink => FILETYPE_SYMBOLIC_LINK,
            crate::sys::fs::FileType::Pipe => FILETYPE_UNKNOWN,
            crate::sys::fs::FileType::Socket => FILETYPE_SOCKET_STREAM,
            crate::sys::fs::FileType::Device => FILETYPE_BLOCK_DEVICE,
        };
        records.push(DirEntryRecord {
            ino: idx as u64,
            name: entry.name,
            file_type,
        });
    }
    Ok(records)
}

// ---------- Preview 2 helpers (memory-agnostic by design) ----------

pub fn open_at(dir_fd: Fd, path: &str, open_flags: u32, create_flags: u32) -> WasiResult<(Fd, u8)> {
    let oflags = if (create_flags & 0x1) != 0 { 0x1 } else { 0 }; // O_CREAT
    let fdflags = if (open_flags & 0x1) != 0 {
        FDFLAGS_APPEND
    } else {
        0
    };

    let rights = RIGHTS_FD_READ | RIGHTS_FD_WRITE | RIGHTS_FD_SEEK | RIGHTS_FD_TELL;

    let fd = path_open(dir_fd, 0, path, oflags, rights, rights, fdflags)?;

    let file_type = {
        let fs_state = FD_TABLE.lock();
        if let Some(file_desc) = fs_state.open_files.get(&fd) {
            file_desc.file_type
        } else {
            FILETYPE_REGULAR_FILE
        }
    };

    Ok((fd, file_type))
}

pub fn read_via_stream(fd: Fd, offset: FileSize) -> WasiResult<super::io::InputStream> {
    let fs_state = FD_TABLE.lock();
    let file_desc = fs_state
        .open_files
        .get(&fd)
        .ok_or_else(WasiError::badf)?;
    if (file_desc.rights_base & RIGHTS_FD_READ) == 0 {
        return Err(WasiError::notcapable());
    }
    let path = file_desc.path.clone();
    drop(fs_state);

    let data = fs::read_file(&path).map_err(|_| WasiError::io())?;
    let start = (offset as usize).min(data.len());
    Ok(super::io::create_input_stream(data[start..].to_vec()))
}

pub fn write_via_stream(fd: Fd, _offset: FileSize) -> WasiResult<super::io::OutputStream> {
    let fs_state = FD_TABLE.lock();
    let file_desc = fs_state
        .open_files
        .get(&fd)
        .ok_or_else(WasiError::badf)?;
    if (file_desc.rights_base & RIGHTS_FD_WRITE) == 0 {
        return Err(WasiError::notcapable());
    }
    Ok(super::io::create_output_stream())
}

pub fn append_via_stream(fd: Fd) -> WasiResult<super::io::OutputStream> {
    let fs_state = FD_TABLE.lock();
    let file_desc = fs_state
        .open_files
        .get(&fd)
        .ok_or_else(WasiError::badf)?;
    if (file_desc.rights_base & RIGHTS_FD_WRITE) == 0 {
        return Err(WasiError::notcapable());
    }
    Ok(super::io::create_output_stream())
}

pub fn list_directory_entries(fd: Fd) -> WasiResult<Vec<String>> {
    let entries = fd_readdir_entries(fd, 0)?;
    Ok(entries.into_iter().map(|e| e.name).collect())
}

pub fn advise(fd: Fd, _offset: FileSize, _len: FileSize, _advice: Advice) -> WasiResult<()> {
    let _ = fd;
    Ok(())
}

pub fn sync_data(_fd: Fd) -> WasiResult<()> {
    fs::sync_filesystem().map_err(|_| WasiError::io())
}

pub fn get_flags(fd: Fd) -> WasiResult<u32> {
    let fs_state = FD_TABLE.lock();
    let file_desc = fs_state
        .open_files
        .get(&fd)
        .ok_or_else(WasiError::badf)?;
    Ok(file_desc.flags as u32)
}

pub fn get_type(fd: Fd) -> WasiResult<u8> {
    let fs_state = FD_TABLE.lock();
    if let Some(file_desc) = fs_state.open_files.get(&fd) {
        Ok(file_desc.file_type)
    } else if fs_state.preopened_dirs.contains_key(&fd) {
        Ok(FILETYPE_DIRECTORY)
    } else {
        Err(WasiError::badf())
    }
}

pub fn set_size(fd: Fd, size: FileSize) -> WasiResult<()> {
    fd_filestat_set_size(fd, size)
}

pub fn set_times(
    _fd: Fd,
    _data_access_timestamp: u64,
    _data_modification_timestamp: u64,
) -> WasiResult<()> {
    // Kernel VFS does not yet expose per-fd time setters; treat as no-op.
    Ok(())
}

pub fn read(fd: Fd, length: FileSize, _offset: FileSize) -> WasiResult<(Vec<u8>, bool)> {
    let data = fd_read(fd, length as usize)?;
    let eof = (data.len() as FileSize) < length;
    Ok((data, eof))
}

pub fn write(fd: Fd, buffer: &[u8], _offset: FileSize) -> WasiResult<FileSize> {
    let n = fd_write(fd, buffer)?;
    Ok(n as FileSize)
}

pub fn read_directory(fd: Fd) -> WasiResult<u32> {
    Ok(fd)
}

pub fn sync(_fd: Fd) -> WasiResult<()> {
    fs::sync_filesystem().map_err(|_| WasiError::io())
}

pub fn create_directory_at(fd: Fd, path: &str) -> WasiResult<()> {
    path_create_directory(fd, path)
}

pub fn stat(fd: Fd, _path_flags: u16, path: &str) -> WasiResult<u64> {
    let fs_state = FD_TABLE.lock();
    let base_path = base_path_of(&fs_state, fd)?;
    drop(fs_state);
    let full_path = resolve_under(&base_path, path).ok_or_else(WasiError::notcapable)?;
    fs::metadata(&full_path).map(|m| m.size).map_err(|_| WasiError::noent())
}

/// Resolve `path` against `fd`'s base, with the same confinement rules used
/// by `path_open`. Returned only — caller does the filestat encoding.
pub fn stat_resolve(fd: Fd, _path_flags: u16, path: &str) -> WasiResult<String> {
    let fs_state = FD_TABLE.lock();
    let base_path = base_path_of(&fs_state, fd)?;
    drop(fs_state);
    resolve_under(&base_path, path).ok_or_else(WasiError::notcapable)
}

pub fn stat_open_directory(fd: Fd, _path_flags: u16, path: &str) -> WasiResult<Fd> {
    let rights = RIGHTS_FD_READ | RIGHTS_PATH_OPEN | RIGHTS_FD_READDIR;
    path_open(fd, 0, path, 0, rights, rights, 0)
}

pub fn link(
    old_fd: Fd,
    _old_path_flags: u16,
    old_path: &str,
    _new_fd: Fd,
    new_path: &str,
) -> WasiResult<()> {
    let fs_state = FD_TABLE.lock();
    let base_path = base_path_of(&fs_state, old_fd)?;
    drop(fs_state);
    let from = resolve_under(&base_path, old_path).ok_or_else(WasiError::notcapable)?;
    let to = resolve_under(&base_path, new_path).ok_or_else(WasiError::notcapable)?;
    let data = fs::read_file(&from).map_err(|_| WasiError::noent())?;
    fs::write_file(&to, data).map_err(|_| WasiError::io())
}

pub fn readlink_at(_fd: Fd, _path: &str) -> WasiResult<String> {
    Err(WasiError::inval())
}

pub fn remove_directory_at(fd: Fd, path: &str) -> WasiResult<()> {
    path_remove_directory(fd, path)
}

pub fn rename_at(fd: Fd, old_path: &str, _new_fd: Fd, new_path: &str) -> WasiResult<()> {
    let fs_state = FD_TABLE.lock();
    let base_path = base_path_of(&fs_state, fd)?;
    drop(fs_state);
    let from = resolve_under(&base_path, old_path).ok_or_else(WasiError::notcapable)?;
    let to = resolve_under(&base_path, new_path).ok_or_else(WasiError::notcapable)?;
    if !fs::exists(&from) {
        return Err(WasiError::noent());
    }
    let data = fs::read_file(&from).map_err(|_| WasiError::io())?;
    fs::write_file(&to, data).map_err(|_| WasiError::io())?;
    fs::remove(&from).map_err(|_| WasiError::io())
}

pub fn symlink_at(_fd: Fd, _old_path: &str, _new_path: &str) -> WasiResult<()> {
    // Kernel VFS supports symlinks at the model level but no free function is
    // exposed yet. Treat as unsupported.
    Err(WasiError::notsup())
}

pub fn unlink_file_at(fd: Fd, path: &str) -> WasiResult<()> {
    path_unlink_file(fd, path)
}

pub fn is_same_object(fd1: Fd, fd2: Fd) -> WasiResult<bool> {
    Ok(fd1 == fd2)
}

pub fn metadata_hash(fd: Fd) -> WasiResult<u64> {
    Ok(fd as u64 * 0x9E3779B97F4A7C15)
}

pub fn metadata_hash_at(fd: Fd, _path_flags: u16, path: &str) -> WasiResult<u64> {
    Ok((fd as u64).wrapping_add(path.len() as u64).wrapping_mul(0x9E3779B97F4A7C15))
}

pub fn drop_descriptor(fd: Fd) -> WasiResult<()> {
    fd_close(fd)
}

// ---------- helpers ----------

fn base_path_of(fs_state: &FilesystemState, fd: Fd) -> WasiResult<String> {
    if let Some(preopen_path) = fs_state.preopened_dirs.get(&fd) {
        Ok(preopen_path.clone())
    } else if let Some(file_desc) = fs_state.open_files.get(&fd) {
        Ok(file_desc.path.clone())
    } else {
        Err(WasiError::badf())
    }
}

/// Resolve `path` against `base_path` and confine the result inside `base_path`
/// (the preopen root or open directory).
///
/// `..` segments that would escape the base return `None`. The kernel VFS does
/// not normalise paths itself, so a literal `/tmp/../etc/hostname` would be
/// interpreted as a top-level lookup that misses the preopen sandbox; this
/// helper canonicalises before handing the path to the VFS.
fn resolve_under(base_path: &str, path: &str) -> Option<String> {
    let combined = if path.starts_with('/') {
        path.to_string()
    } else if base_path == "/" {
        format!("/{}", path)
    } else {
        format!("{}/{}", base_path, path)
    };

    let normalised = normalise_path(&combined);

    // The base path itself may need normalising (e.g. "//"); collapse for the
    // confinement check.
    let base_norm = normalise_path(base_path);
    if base_norm != "/" && !(normalised == base_norm
        || normalised.starts_with(&format!("{}/", base_norm)))
    {
        return None;
    }

    Some(normalised)
}

fn normalise_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    if parts.is_empty() {
        "/".to_string()
    } else {
        let mut out = String::new();
        for p in &parts {
            out.push('/');
            out.push_str(p);
        }
        out
    }
}
