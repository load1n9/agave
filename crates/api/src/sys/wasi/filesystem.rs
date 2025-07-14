// WASI Filesystem implementation for Agave OS
use super::super::fs;
use super::error::*;
use super::types::*;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use spin::Mutex;

// Global filesystem state
static FILESYSTEM: Mutex<FilesystemState> = Mutex::new(FilesystemState::new());

#[derive(Debug)]
pub struct FilesystemState {
    open_files: BTreeMap<Fd, FileDescriptor>,
    preopened_dirs: BTreeMap<Fd, String>,
    next_fd: Fd,
    cwd: String,
}

impl FilesystemState {
    pub const fn new() -> Self {
        Self {
            open_files: BTreeMap::new(),
            preopened_dirs: BTreeMap::new(),
            next_fd: 3, // Start after stdin(0), stdout(1), stderr(2)
            cwd: String::new(),
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
    pub data: Vec<u8>,
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
            data: Vec::new(),
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
            data: Vec::new(),
            is_directory: true,
        }
    }
}

// Initialize filesystem with standard preopened directories
pub fn init_filesystem() {
    let mut fs = FILESYSTEM.lock();
    fs.add_preopen("/".to_string());
    fs.add_preopen("/tmp".to_string());
    fs.cwd = "/".to_string();
}

// Preview 1 API implementations
pub fn fd_prestat_get(fd: Fd) -> WasiResult<Prestat> {
    let fs = FILESYSTEM.lock();

    if let Some(path) = fs.preopened_dirs.get(&fd) {
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

pub fn fd_prestat_dir_name(fd: Fd, path_ptr: u32, path_len: Size) -> WasiResult<()> {
    let fs = FILESYSTEM.lock();

    if let Some(path) = fs.preopened_dirs.get(&fd) {
        if path.len() > path_len as usize {
            return Err(WasiError::nametoolong());
        }
        // In a real implementation, we would write to the WebAssembly memory
        // For now, we'll just validate the operation
        Ok(())
    } else {
        Err(WasiError::badf())
    }
}

pub fn path_open(
    fd: Fd,
    dirflags: LookupFlags,
    path: &str,
    oflags: OFlags,
    fs_rights_base: Rights,
    fs_rights_inheriting: Rights,
    fdflags: FdFlags,
) -> WasiResult<Fd> {
    let mut fs_state = FILESYSTEM.lock();

    // Check if the directory fd exists and has the required rights
    if !fs_state.preopened_dirs.contains_key(&fd) && !fs_state.open_files.contains_key(&fd) {
        return Err(WasiError::badf());
    }

    // Create the full path
    let base_path = if let Some(preopen_path) = fs_state.preopened_dirs.get(&fd) {
        preopen_path.clone()
    } else if let Some(file_desc) = fs_state.open_files.get(&fd) {
        if !file_desc.is_directory {
            return Err(WasiError::notdir());
        }
        file_desc.path.clone()
    } else {
        return Err(WasiError::badf());
    };

    let full_path = if path.starts_with('/') {
        path.to_string()
    } else if base_path == "/" {
        format!("/{}", path)
    } else {
        format!("{}/{}", base_path, path)
    };

    // Try to read the file using the actual filesystem
    let file_data = match fs::read_file(&full_path) {
        Ok(data) => data,
        Err(_) => {
            // If file doesn't exist and O_CREAT is set, create it
            if (oflags & 0x1) != 0 {
                // O_CREAT
                Vec::new()
            } else {
                return Err(WasiError::noent());
            }
        }
    };

    let new_fd = fs_state.allocate_fd();
    let mut file_desc =
        FileDescriptor::new(full_path, fdflags, fs_rights_base, fs_rights_inheriting);
    file_desc.data = file_data;
    file_desc.size = file_desc.data.len() as FileSize;

    fs_state.open_files.insert(new_fd, file_desc);
    Ok(new_fd)
}

pub fn fd_read(fd: Fd, iovs: &[IOVec]) -> WasiResult<Size> {
    let mut fs_state = FILESYSTEM.lock();

    if let Some(file_desc) = fs_state.open_files.get_mut(&fd) {
        if (file_desc.rights_base & RIGHTS_FD_READ) == 0 {
            return Err(WasiError::notcapable());
        }

        let mut total_read = 0;

        for iov in iovs {
            let bytes_to_read = iov.buf_len.min((file_desc.size - file_desc.offset) as u32);
            if bytes_to_read == 0 {
                break;
            }

            // In a real implementation, we would write to WebAssembly memory at iov.buf
            // For now, we'll just simulate the read
            file_desc.offset += bytes_to_read as FileSize;
            total_read += bytes_to_read;
        }

        Ok(total_read)
    } else {
        Err(WasiError::badf())
    }
}

pub fn fd_write(fd: Fd, iovs: &[CIOVec]) -> WasiResult<Size> {
    let mut fs_state = FILESYSTEM.lock();

    if let Some(file_desc) = fs_state.open_files.get_mut(&fd) {
        if (file_desc.rights_base & RIGHTS_FD_WRITE) == 0 {
            return Err(WasiError::notcapable());
        }

        let mut total_written = 0;

        for iov in iovs {
            // In a real implementation, we would read from WebAssembly memory at iov.buf
            // For now, we'll simulate writing zeros
            let bytes_to_write = iov.buf_len;

            // Extend the file data if necessary
            let new_end = file_desc.offset + bytes_to_write as FileSize;
            if new_end > file_desc.size {
                file_desc.data.resize(new_end as usize, 0);
                file_desc.size = new_end;
            }

            file_desc.offset += bytes_to_write as FileSize;
            total_written += bytes_to_write;
        }

        Ok(total_written)
    } else {
        Err(WasiError::badf())
    }
}

pub fn fd_seek(fd: Fd, offset: FileDelta, whence: Whence) -> WasiResult<FileSize> {
    let mut fs_state = FILESYSTEM.lock();

    if let Some(file_desc) = fs_state.open_files.get_mut(&fd) {
        if (file_desc.rights_base & RIGHTS_FD_SEEK) == 0 {
            return Err(WasiError::notcapable());
        }

        let new_offset = match whence {
            0 => offset as FileSize,                             // SEEK_SET
            1 => file_desc.offset.saturating_add_signed(offset), // SEEK_CUR
            2 => file_desc.size.saturating_add_signed(offset),   // SEEK_END
            _ => return Err(WasiError::inval()),
        };

        file_desc.offset = new_offset;
        Ok(new_offset)
    } else {
        Err(WasiError::badf())
    }
}

pub fn fd_tell(fd: Fd) -> WasiResult<FileSize> {
    let fs_state = FILESYSTEM.lock();

    if let Some(file_desc) = fs_state.open_files.get(&fd) {
        if (file_desc.rights_base & RIGHTS_FD_TELL) == 0 {
            return Err(WasiError::notcapable());
        }
        Ok(file_desc.offset)
    } else {
        Err(WasiError::badf())
    }
}

pub fn fd_close(fd: Fd) -> WasiResult<()> {
    let mut fs_state = FILESYSTEM.lock();

    if fs_state.open_files.remove(&fd).is_some() {
        Ok(())
    } else {
        Err(WasiError::badf())
    }
}

pub fn fd_sync(fd: Fd) -> WasiResult<()> {
    let fs_state = FILESYSTEM.lock();

    if let Some(file_desc) = fs_state.open_files.get(&fd) {
        if (file_desc.rights_base & RIGHTS_FD_SYNC) == 0 {
            return Err(WasiError::notcapable());
        }
        // In a real implementation, this would flush to disk
        Ok(())
    } else {
        Err(WasiError::badf())
    }
}

pub fn fd_datasync(fd: Fd) -> WasiResult<()> {
    let fs_state = FILESYSTEM.lock();

    if let Some(file_desc) = fs_state.open_files.get(&fd) {
        if (file_desc.rights_base & RIGHTS_FD_DATASYNC) == 0 {
            return Err(WasiError::notcapable());
        }
        // In a real implementation, this would flush data to disk
        Ok(())
    } else {
        Err(WasiError::badf())
    }
}

pub fn fd_allocate(fd: Fd, offset: FileSize, len: FileSize) -> WasiResult<()> {
    let mut fs_state = FILESYSTEM.lock();

    if let Some(file_desc) = fs_state.open_files.get_mut(&fd) {
        if (file_desc.rights_base & RIGHTS_FD_ALLOCATE) == 0 {
            return Err(WasiError::notcapable());
        }

        let new_size = offset + len;
        if new_size > file_desc.size {
            file_desc.data.resize(new_size as usize, 0);
            file_desc.size = new_size;
        }
        Ok(())
    } else {
        Err(WasiError::badf())
    }
}

pub fn fd_advise(fd: Fd, offset: FileSize, len: FileSize, advice: Advice) -> WasiResult<()> {
    let fs_state = FILESYSTEM.lock();

    if let Some(file_desc) = fs_state.open_files.get(&fd) {
        if (file_desc.rights_base & RIGHTS_FD_ADVISE) == 0 {
            return Err(WasiError::notcapable());
        }
        // Advice is just a hint, so we always succeed
        Ok(())
    } else {
        Err(WasiError::badf())
    }
}

pub fn fd_fdstat_get(fd: Fd) -> WasiResult<FdStat> {
    let fs_state = FILESYSTEM.lock();

    if let Some(file_desc) = fs_state.open_files.get(&fd) {
        // Create fdstat structure
        let mut fdstat = [0u8; 24];
        fdstat[0] = file_desc.file_type;
        // flags (2 bytes at offset 2)
        fdstat[2..4].copy_from_slice(&file_desc.flags.to_le_bytes());
        // rights_base (8 bytes at offset 8)
        fdstat[8..16].copy_from_slice(&file_desc.rights_base.to_le_bytes());
        // rights_inheriting (8 bytes at offset 16)
        fdstat[16..24].copy_from_slice(&file_desc.rights_inheriting.to_le_bytes());
        Ok(fdstat)
    } else if fs_state.preopened_dirs.contains_key(&fd) {
        // Preopened directory
        let mut fdstat = [0u8; 24];
        fdstat[0] = FILETYPE_DIRECTORY;
        let rights = RIGHTS_FD_READ | RIGHTS_PATH_OPEN | RIGHTS_FD_READDIR;
        fdstat[8..16].copy_from_slice(&rights.to_le_bytes());
        fdstat[16..24].copy_from_slice(&rights.to_le_bytes());
        Ok(fdstat)
    } else {
        Err(WasiError::badf())
    }
}

pub fn fd_fdstat_set_flags(fd: Fd, flags: FdFlags) -> WasiResult<()> {
    let mut fs_state = FILESYSTEM.lock();

    if let Some(file_desc) = fs_state.open_files.get_mut(&fd) {
        if (file_desc.rights_base & RIGHTS_FD_FDSTAT_SET_FLAGS) == 0 {
            return Err(WasiError::notcapable());
        }
        file_desc.flags = flags;
        Ok(())
    } else {
        Err(WasiError::badf())
    }
}

pub fn fd_filestat_get(fd: Fd) -> WasiResult<FileStat> {
    let fs_state = FILESYSTEM.lock();

    if let Some(file_desc) = fs_state.open_files.get(&fd) {
        if (file_desc.rights_base & RIGHTS_FD_FILESTAT_GET) == 0 {
            return Err(WasiError::notcapable());
        }

        let mut filestat = [0u8; 56];
        // dev (8 bytes at offset 0)
        filestat[0..8].copy_from_slice(&1u64.to_le_bytes());
        // ino (8 bytes at offset 8)
        filestat[8..16].copy_from_slice(&(fd as u64).to_le_bytes());
        // filetype (1 byte at offset 16)
        filestat[16] = file_desc.file_type;
        // nlink (8 bytes at offset 24)
        filestat[24..32].copy_from_slice(&1u64.to_le_bytes());
        // size (8 bytes at offset 32)
        filestat[32..40].copy_from_slice(&file_desc.size.to_le_bytes());
        // atim, mtim, ctim (8 bytes each at offsets 40, 48, 56)
        let current_time = super::clocks::clock_time_get(CLOCKID_REALTIME, 0).unwrap_or(0);
        filestat[40..48].copy_from_slice(&current_time.to_le_bytes());
        filestat[48..56].copy_from_slice(&current_time.to_le_bytes());

        Ok(filestat)
    } else {
        Err(WasiError::badf())
    }
}

pub fn fd_filestat_set_size(fd: Fd, size: FileSize) -> WasiResult<()> {
    let mut fs_state = FILESYSTEM.lock();

    if let Some(file_desc) = fs_state.open_files.get_mut(&fd) {
        if (file_desc.rights_base & RIGHTS_FD_FILESTAT_SET_SIZE) == 0 {
            return Err(WasiError::notcapable());
        }

        file_desc.data.resize(size as usize, 0);
        file_desc.size = size;

        if file_desc.offset > size {
            file_desc.offset = size;
        }

        Ok(())
    } else {
        Err(WasiError::badf())
    }
}

pub fn path_create_directory(fd: Fd, path: &str) -> WasiResult<()> {
    let fs_state = FILESYSTEM.lock();

    // Check directory permissions
    if !fs_state.preopened_dirs.contains_key(&fd) {
        if let Some(file_desc) = fs_state.open_files.get(&fd) {
            if !file_desc.is_directory
                || (file_desc.rights_base & RIGHTS_PATH_CREATE_DIRECTORY) == 0
            {
                return Err(WasiError::notcapable());
            }
        } else {
            return Err(WasiError::badf());
        }
    }

    // In a real implementation, this would create the directory
    // For now, we'll just validate the operation
    Ok(())
}

pub fn path_unlink_file(fd: Fd, path: &str) -> WasiResult<()> {
    let fs_state = FILESYSTEM.lock();

    // Check directory permissions
    if !fs_state.preopened_dirs.contains_key(&fd) {
        if let Some(file_desc) = fs_state.open_files.get(&fd) {
            if !file_desc.is_directory || (file_desc.rights_base & RIGHTS_PATH_UNLINK_FILE) == 0 {
                return Err(WasiError::notcapable());
            }
        } else {
            return Err(WasiError::badf());
        }
    }

    // In a real implementation, this would delete the file
    // For now, we'll just validate the operation
    Ok(())
}

pub fn path_remove_directory(fd: Fd, path: &str) -> WasiResult<()> {
    let fs_state = FILESYSTEM.lock();

    // Check directory permissions
    if !fs_state.preopened_dirs.contains_key(&fd) {
        if let Some(file_desc) = fs_state.open_files.get(&fd) {
            if !file_desc.is_directory
                || (file_desc.rights_base & RIGHTS_PATH_REMOVE_DIRECTORY) == 0
            {
                return Err(WasiError::notcapable());
            }
        } else {
            return Err(WasiError::badf());
        }
    }

    // In a real implementation, this would remove the directory
    // For now, we'll just validate the operation
    Ok(())
}

pub fn fd_readdir(fd: Fd, buf: &mut [u8], cookie: DirCookie) -> WasiResult<Size> {
    let fs_state = FILESYSTEM.lock();

    if let Some(file_desc) = fs_state.open_files.get(&fd) {
        if !file_desc.is_directory || (file_desc.rights_base & RIGHTS_FD_READDIR) == 0 {
            return Err(WasiError::notdir());
        }
    } else if !fs_state.preopened_dirs.contains_key(&fd) {
        return Err(WasiError::badf());
    }

    // In a real implementation, this would read directory entries
    // For now, we'll return empty directory
    Ok(0)
}

// Preview 2 API extensions
pub fn open_at(dir_fd: Fd, path: &str, open_flags: u32, create_flags: u32) -> WasiResult<(Fd, u8)> {
    // Convert Preview 2 flags to Preview 1 flags
    let oflags = if (create_flags & 0x1) != 0 { 0x1 } else { 0 }; // O_CREAT
    let fdflags = if (open_flags & 0x1) != 0 {
        FDFLAGS_APPEND
    } else {
        0
    };

    let rights = RIGHTS_FD_READ | RIGHTS_FD_WRITE | RIGHTS_FD_SEEK | RIGHTS_FD_TELL;

    let fd = path_open(dir_fd, 0, path, oflags, rights, rights, fdflags)?;

    // Return file descriptor and file type
    let file_type = {
        let fs_state = FILESYSTEM.lock();
        if let Some(file_desc) = fs_state.open_files.get(&fd) {
            file_desc.file_type
        } else {
            FILETYPE_REGULAR_FILE
        }
    };

    Ok((fd, file_type))
}

pub fn read_via_stream(fd: Fd, offset: FileSize) -> WasiResult<super::io::InputStream> {
    let mut fs_state = FILESYSTEM.lock();

    if let Some(file_desc) = fs_state.open_files.get_mut(&fd) {
        if (file_desc.rights_base & RIGHTS_FD_READ) == 0 {
            return Err(WasiError::notcapable());
        }

        // Create a stream from the file data starting at the offset
        let start = offset.min(file_desc.size) as usize;
        let data = file_desc.data[start..].to_vec();

        Ok(super::io::create_input_stream(data))
    } else {
        Err(WasiError::badf())
    }
}

pub fn write_via_stream(fd: Fd, offset: FileSize) -> WasiResult<super::io::OutputStream> {
    let fs_state = FILESYSTEM.lock();

    if let Some(file_desc) = fs_state.open_files.get(&fd) {
        if (file_desc.rights_base & RIGHTS_FD_WRITE) == 0 {
            return Err(WasiError::notcapable());
        }

        // Create an output stream for the file
        Ok(super::io::create_output_stream())
    } else {
        Err(WasiError::badf())
    }
}

pub fn append_via_stream(fd: Fd) -> WasiResult<super::io::OutputStream> {
    let fs_state = FILESYSTEM.lock();

    if let Some(file_desc) = fs_state.open_files.get(&fd) {
        if (file_desc.rights_base & RIGHTS_FD_WRITE) == 0 {
            return Err(WasiError::notcapable());
        }

        // Create an output stream for appending to the file
        Ok(super::io::create_output_stream())
    } else {
        Err(WasiError::badf())
    }
}

// Additional functions for demo compatibility
pub fn list_directory_entries(fd: Fd) -> WasiResult<Vec<String>> {
    // Basic implementation - in a real OS this would read actual directory entries
    log::debug!("list_directory_entries({})", fd);
    use alloc::vec;
    Ok(vec![
        ".".to_string(),
        "..".to_string(),
        "file1.txt".to_string(),
        "file2.txt".to_string(),
    ])
}

// Additional filesystem functions for Preview 2 compatibility
pub fn advise(fd: Fd, offset: FileSize, len: FileSize, advice: Advice) -> WasiResult<()> {
    // For demo, just pretend to advise
    log::info!(
        "Advising fd {} offset {} len {} advice {:?}",
        fd,
        offset,
        len,
        advice
    );
    Ok(())
}

pub fn sync_data(fd: Fd) -> WasiResult<()> {
    // For demo, just pretend to sync
    log::info!("Syncing data for fd {}", fd);
    Ok(())
}

pub fn get_flags(fd: Fd) -> WasiResult<u32> {
    // Return some default flags
    Ok(0)
}

pub fn get_type(fd: Fd) -> WasiResult<u8> {
    // Return regular file type
    Ok(4) // FILETYPE_REGULAR_FILE
}

pub fn set_size(fd: Fd, size: FileSize) -> WasiResult<()> {
    // For demo, just pretend to set size
    log::info!("Setting size of fd {} to {}", fd, size);
    Ok(())
}

pub fn set_times(
    fd: Fd,
    data_access_timestamp: u64,
    data_modification_timestamp: u64,
) -> WasiResult<()> {
    // For demo, just pretend to set times
    log::info!(
        "Setting times for fd {} access {} modify {}",
        fd,
        data_access_timestamp,
        data_modification_timestamp
    );
    Ok(())
}

pub fn read(fd: Fd, length: FileSize, offset: FileSize) -> WasiResult<(Vec<u8>, bool)> {
    // For demo, return empty data
    let data = alloc::vec![0u8; length.min(1024) as usize];
    Ok((data, true)) // true = end of file
}

pub fn write(fd: Fd, buffer: &[u8], offset: FileSize) -> WasiResult<FileSize> {
    // For demo, pretend to write all bytes
    log::info!(
        "Writing {} bytes to fd {} at offset {}",
        buffer.len(),
        fd,
        offset
    );
    Ok(buffer.len() as FileSize)
}

pub fn read_directory(fd: Fd) -> WasiResult<u32> {
    // Return a dummy directory stream ID
    Ok(fd)
}

pub fn sync(fd: Fd) -> WasiResult<()> {
    // For demo, just pretend to sync
    log::info!("Syncing fd {}", fd);
    Ok(())
}

pub fn create_directory_at(fd: Fd, path: &str) -> WasiResult<()> {
    // For demo, just pretend to create directory
    log::info!("Creating directory {} at fd {}", path, fd);
    Ok(())
}

pub fn stat(fd: Fd, path_flags: u16, path: &str) -> WasiResult<u64> {
    // Return dummy stat data
    log::info!("Stat {} at fd {} with flags {}", path, fd, path_flags);
    Ok(0x1000) // Dummy stat
}

pub fn stat_open_directory(fd: Fd, path_flags: u16, path: &str) -> WasiResult<Fd> {
    // Return dummy directory fd
    log::info!(
        "Stat open directory {} at fd {} with flags {}",
        path,
        fd,
        path_flags
    );
    Ok(fd + 1)
}

pub fn link(
    fd: Fd,
    old_path_flags: u16,
    old_path: &str,
    new_fd: Fd,
    new_path: &str,
) -> WasiResult<()> {
    // For demo, just pretend to create link
    log::info!(
        "Link {} (fd {}) to {} (fd {})",
        old_path,
        fd,
        new_path,
        new_fd
    );
    Ok(())
}

pub fn readlink_at(fd: Fd, path: &str) -> WasiResult<String> {
    // Return dummy link target
    log::info!("Readlink {} at fd {}", path, fd);
    Ok("/dummy/target".to_string())
}

pub fn remove_directory_at(fd: Fd, path: &str) -> WasiResult<()> {
    // For demo, just pretend to remove directory
    log::info!("Removing directory {} at fd {}", path, fd);
    Ok(())
}

pub fn rename_at(fd: Fd, old_path: &str, new_fd: Fd, new_path: &str) -> WasiResult<()> {
    // For demo, just pretend to rename
    log::info!(
        "Rename {} (fd {}) to {} (fd {})",
        old_path,
        fd,
        new_path,
        new_fd
    );
    Ok(())
}

pub fn symlink_at(fd: Fd, old_path: &str, new_path: &str) -> WasiResult<()> {
    // For demo, just pretend to create symlink
    log::info!("Symlink {} to {} at fd {}", old_path, new_path, fd);
    Ok(())
}

pub fn unlink_file_at(fd: Fd, path: &str) -> WasiResult<()> {
    // For demo, just pretend to unlink file
    log::info!("Unlink file {} at fd {}", path, fd);
    Ok(())
}

pub fn is_same_object(fd1: Fd, fd2: Fd) -> WasiResult<bool> {
    // For demo, just compare fds
    Ok(fd1 == fd2)
}

pub fn metadata_hash(fd: Fd) -> WasiResult<u64> {
    // Return dummy hash
    Ok(fd as u64 * 0x123456789)
}

pub fn metadata_hash_at(fd: Fd, path_flags: u16, path: &str) -> WasiResult<u64> {
    // Return dummy hash based on path
    log::info!(
        "Metadata hash for {} at fd {} with flags {}",
        path,
        fd,
        path_flags
    );
    Ok(path.len() as u64 * 0x987654321)
}

pub fn drop_descriptor(fd: Fd) -> WasiResult<()> {
    // For demo, just pretend to drop descriptor
    log::info!("Dropping descriptor {}", fd);
    Ok(())
}
