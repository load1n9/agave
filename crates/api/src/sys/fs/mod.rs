/// Enhanced filesystem implementation for Agave OS
/// Supports virtual filesystem with multiple backends
use crate::sys::error::{AgaveError, AgaveResult, FsError};
use alloc::{
    boxed::Box,
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;
use spin::Mutex;

/// File system types
#[derive(Debug, Clone, PartialEq)]
pub enum FileSystemType {
    Virtual, // In-memory filesystem
    TarFs,   // Read-only tar archive
    RamDisk, // RAM-based disk
    Network, // Network filesystem (future)
}

/// File types
#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    Regular,
    Directory,
    Symlink,
    Device,
    Pipe,
    Socket,
}

/// File permissions (Unix-style)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilePermissions {
    pub owner_read: bool,
    pub owner_write: bool,
    pub owner_execute: bool,
    pub group_read: bool,
    pub group_write: bool,
    pub group_execute: bool,
    pub other_read: bool,
    pub other_write: bool,
    pub other_execute: bool,
}

impl Default for FilePermissions {
    fn default() -> Self {
        Self {
            owner_read: true,
            owner_write: true,
            owner_execute: false,
            group_read: true,
            group_write: false,
            group_execute: false,
            other_read: true,
            other_write: false,
            other_execute: false,
        }
    }
}

impl FilePermissions {
    pub fn octal(&self) -> u16 {
        let mut mode = 0;
        if self.owner_read {
            mode |= 0o400;
        }
        if self.owner_write {
            mode |= 0o200;
        }
        if self.owner_execute {
            mode |= 0o100;
        }
        if self.group_read {
            mode |= 0o040;
        }
        if self.group_write {
            mode |= 0o020;
        }
        if self.group_execute {
            mode |= 0o010;
        }
        if self.other_read {
            mode |= 0o004;
        }
        if self.other_write {
            mode |= 0o002;
        }
        if self.other_execute {
            mode |= 0o001;
        }
        mode
    }
}

/// File metadata
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub file_type: FileType,
    pub size: u64,
    pub permissions: FilePermissions,
    pub created_time: u64,
    pub modified_time: u64,
    pub accessed_time: u64,
    pub uid: u32,
    pub gid: u32,
}

impl Default for FileMetadata {
    fn default() -> Self {
        let now = crate::sys::interrupts::TIME_MS.load(core::sync::atomic::Ordering::Relaxed);
        Self {
            file_type: FileType::Regular,
            size: 0,
            permissions: FilePermissions::default(),
            created_time: now,
            modified_time: now,
            accessed_time: now,
            uid: 0,
            gid: 0,
        }
    }
}

/// Directory entry
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub file_type: FileType,
    pub size: u64,
}

/// File handle for open files
#[derive(Debug)]
pub struct FileHandle {
    pub path: String,
    pub position: u64,
    pub readable: bool,
    pub writable: bool,
    pub metadata: FileMetadata,
}

/// Virtual filesystem node
#[derive(Debug, Clone)]
pub enum VfsNode {
    File {
        metadata: FileMetadata,
        content: Vec<u8>,
    },
    Directory {
        metadata: FileMetadata,
        children: BTreeMap<String, VfsNode>,
    },
    Symlink {
        metadata: FileMetadata,
        target: String,
    },
}

impl VfsNode {
    pub fn new_file(content: Vec<u8>) -> Self {
        let mut metadata = FileMetadata::default();
        metadata.file_type = FileType::Regular;
        metadata.size = content.len() as u64;

        VfsNode::File { metadata, content }
    }

    pub fn new_directory() -> Self {
        let mut metadata = FileMetadata::default();
        metadata.file_type = FileType::Directory;
        metadata.permissions.owner_execute = true;
        metadata.permissions.group_execute = true;
        metadata.permissions.other_execute = true;

        VfsNode::Directory {
            metadata,
            children: BTreeMap::new(),
        }
    }

    pub fn new_symlink(target: String) -> Self {
        let mut metadata = FileMetadata::default();
        metadata.file_type = FileType::Symlink;
        metadata.size = target.len() as u64;

        VfsNode::Symlink { metadata, target }
    }

    pub fn metadata(&self) -> &FileMetadata {
        match self {
            VfsNode::File { metadata, .. } => metadata,
            VfsNode::Directory { metadata, .. } => metadata,
            VfsNode::Symlink { metadata, .. } => metadata,
        }
    }

    pub fn metadata_mut(&mut self) -> &mut FileMetadata {
        match self {
            VfsNode::File { metadata, .. } => metadata,
            VfsNode::Directory { metadata, .. } => metadata,
            VfsNode::Symlink { metadata, .. } => metadata,
        }
    }
}

/// Virtual file system
pub struct VirtualFileSystem {
    root: VfsNode,
    open_files: BTreeMap<u64, FileHandle>,
    next_fd: u64,
}

impl VirtualFileSystem {
    pub fn new() -> Self {
        let mut vfs = Self {
            root: VfsNode::new_directory(),
            open_files: BTreeMap::new(),
            next_fd: 3, // Start after stdin(0), stdout(1), stderr(2)
        };

        // Create standard directories
        vfs.create_standard_directories();
        vfs.populate_demo_files();

        vfs
    }

    fn create_standard_directories(&mut self) {
        let directories = [
            "/bin",
            "/etc",
            "/home",
            "/home/user",
            "/usr",
            "/usr/bin",
            "/var",
            "/var/log",
            "/tmp",
            "/dev",
            "/proc",
            "/sys",
        ];

        for dir in &directories {
            if let Err(e) = self.create_dir_all(dir) {
                log::warn!("Failed to create directory {}: {:?}", dir, e);
            }
        }
    }

    fn populate_demo_files(&mut self) {
        // Demo files for the terminal
        let files: &[(&str, &[u8])] = &[
            ("/etc/hostname", b"agave-os\n"),
            ("/etc/version", b"Agave OS v1.0.0\n"),
            (
                "/home/user/.bashrc",
                b"# Agave OS bash configuration\necho 'Welcome to Agave OS!'\n",
            ),
            ("/var/log/system.log", b"System log initialized\n"),
            ("/tmp/readme.txt", b"This is a temporary file\n"),
            ("/proc/version", b"Agave OS 1.0.0 (x86_64)\n"),
            ("/proc/meminfo", b"MemTotal: 104857600\nMemFree: 52428800\n"),
            ("/proc/cpuinfo", b"processor: 0\nmodel name: Virtual CPU\n"),
        ];

        for (path, content) in files {
            if let Err(e) = self.write_file(path, content.to_vec()) {
                log::warn!("Failed to create file {}: {:?}", path, e);
            }
        }

        // Create some demo binary files
        let binaries = [
            "/bin/ls",
            "/bin/cat",
            "/bin/echo",
            "/bin/grep",
            "/bin/ps",
            "/usr/bin/top",
            "/usr/bin/nano",
            "/usr/bin/vim",
        ];

        for binary in &binaries {
            let content = format!("#!/bin/sh\necho 'Binary: {}'\n", binary).into_bytes();
            if let Err(e) = self.write_file(binary, content) {
                log::warn!("Failed to create binary {}: {:?}", binary, e);
            } else {
                // Make executable
                if let Ok(mut node) = self.get_node_mut(binary) {
                    let metadata = node.metadata_mut();
                    metadata.permissions.owner_execute = true;
                    metadata.permissions.group_execute = true;
                    metadata.permissions.other_execute = true;
                }
            }
        }
    }

    /// Open a file and return file descriptor
    pub fn open(&mut self, path: &str, readable: bool, writable: bool) -> AgaveResult<u64> {
        let node = self.get_node(path)?;
        let metadata = node.metadata().clone();

        if matches!(metadata.file_type, FileType::Directory) && (readable || writable) {
            return Err(AgaveError::FileSystemError(FsError::IsDirectory));
        }

        let fd = self.next_fd;
        self.next_fd += 1;

        let handle = FileHandle {
            path: path.to_string(),
            position: 0,
            readable,
            writable,
            metadata,
        };

        self.open_files.insert(fd, handle);
        Ok(fd)
    }

    /// Close a file descriptor
    pub fn close(&mut self, fd: u64) -> AgaveResult<()> {
        self.open_files
            .remove(&fd)
            .ok_or(AgaveError::FileSystemError(FsError::InvalidFileDescriptor))?;
        Ok(())
    }

    /// Read from a file descriptor
    pub fn read(&mut self, fd: u64, buffer: &mut [u8]) -> AgaveResult<usize> {
        // First get the path from the handle
        let path = {
            let handle = self
                .open_files
                .get(&fd)
                .ok_or(AgaveError::FileSystemError(FsError::InvalidFileDescriptor))?;

            if !handle.readable {
                return Err(AgaveError::PermissionDenied);
            }

            handle.path.clone()
        };

        let node = self.get_node(&path)?;

        match node {
            VfsNode::File { content, .. } => {
                let start = {
                    let handle = self.open_files.get(&fd).unwrap(); // Safe since we checked above
                    handle.position as usize
                };
                let end = (start + buffer.len()).min(content.len());

                if start >= content.len() {
                    return Ok(0); // EOF
                }

                let bytes_read = end - start;
                buffer[..bytes_read].copy_from_slice(&content[start..end]);

                // Update position after the read
                let handle = self.open_files.get_mut(&fd).unwrap();
                handle.position += bytes_read as u64;

                Ok(bytes_read)
            }
            _ => Err(AgaveError::FileSystemError(FsError::IsDirectory)),
        }
    }

    /// Write to a file descriptor
    pub fn write(&mut self, fd: u64, data: &[u8]) -> AgaveResult<usize> {
        // First get path and position without holding a mutable reference
        let (path, position) = {
            let handle = self
                .open_files
                .get(&fd)
                .ok_or(AgaveError::FileSystemError(FsError::InvalidFileDescriptor))?;

            if !handle.writable {
                return Err(AgaveError::PermissionDenied);
            }

            (handle.path.clone(), handle.position)
        };

        // Modify the file content
        let result = {
            let node = self.get_node_mut(&path)?;

            match node {
                VfsNode::File {
                    content, metadata, ..
                } => {
                    let end_pos = position as usize + data.len();

                    // Extend content if necessary
                    if end_pos > content.len() {
                        content.resize(end_pos, 0);
                    }

                    // Write data
                    content[position as usize..end_pos].copy_from_slice(data);

                    // Update metadata
                    metadata.size = content.len() as u64;
                    metadata.modified_time =
                        crate::sys::interrupts::TIME_MS.load(core::sync::atomic::Ordering::Relaxed);

                    Ok((data.len(), metadata.clone()))
                }
                _ => Err(AgaveError::FileSystemError(FsError::IsDirectory)),
            }
        };

        // Update handle after successful write
        match result {
            Ok((bytes_written, metadata)) => {
                let handle = self.open_files.get_mut(&fd).unwrap();
                handle.position += bytes_written as u64;
                handle.metadata = metadata;
                Ok(bytes_written)
            }
            Err(e) => Err(e),
        }
    }

    /// List directory contents
    pub fn read_dir(&self, path: &str) -> AgaveResult<Vec<DirEntry>> {
        let node = self.get_node(path)?;

        match node {
            VfsNode::Directory { children, .. } => {
                let mut entries = Vec::new();

                // Add . and .. entries
                entries.push(DirEntry {
                    name: ".".to_string(),
                    file_type: FileType::Directory,
                    size: 0,
                });

                if path != "/" {
                    entries.push(DirEntry {
                        name: "..".to_string(),
                        file_type: FileType::Directory,
                        size: 0,
                    });
                }

                // Add actual entries
                for (name, child) in children {
                    entries.push(DirEntry {
                        name: name.clone(),
                        file_type: child.metadata().file_type.clone(),
                        size: child.metadata().size,
                    });
                }

                Ok(entries)
            }
            _ => Err(AgaveError::FileSystemError(FsError::NotDirectory)),
        }
    }

    /// Get file metadata
    pub fn metadata(&self, path: &str) -> AgaveResult<FileMetadata> {
        let node = self.get_node(path)?;
        Ok(node.metadata().clone())
    }

    /// Create a directory
    pub fn create_dir(&mut self, path: &str) -> AgaveResult<()> {
        if self.get_node(path).is_ok() {
            return Err(AgaveError::AlreadyExists);
        }

        let parent_path = get_parent_path(path);
        let filename = get_filename(path);

        let parent = self.get_node_mut(&parent_path)?;

        match parent {
            VfsNode::Directory { children, .. } => {
                children.insert(filename.to_string(), VfsNode::new_directory());
                Ok(())
            }
            _ => Err(AgaveError::FileSystemError(FsError::NotDirectory)),
        }
    }

    /// Create directories recursively
    pub fn create_dir_all(&mut self, path: &str) -> AgaveResult<()> {
        let parts: Vec<&str> = path
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        let mut current_path = String::new();

        for part in parts {
            current_path.push('/');
            current_path.push_str(part);

            if self.get_node(&current_path).is_err() {
                self.create_dir(&current_path)?;
            }
        }

        Ok(())
    }

    /// Remove a file or directory
    pub fn remove(&mut self, path: &str) -> AgaveResult<()> {
        if path == "/" {
            return Err(AgaveError::PermissionDenied);
        }

        let parent_path = get_parent_path(path);
        let filename = get_filename(path);

        let parent = self.get_node_mut(&parent_path)?;

        match parent {
            VfsNode::Directory { children, .. } => {
                children.remove(filename).ok_or(AgaveError::NotFound)?;
                Ok(())
            }
            _ => Err(AgaveError::FileSystemError(FsError::NotDirectory)),
        }
    }

    /// Write entire file content
    pub fn write_file(&mut self, path: &str, content: Vec<u8>) -> AgaveResult<()> {
        let parent_path = get_parent_path(path);
        let filename = get_filename(path);

        // Ensure parent directory exists
        if self.get_node(&parent_path).is_err() {
            self.create_dir_all(&parent_path)?;
        }

        let parent = self.get_node_mut(&parent_path)?;

        match parent {
            VfsNode::Directory { children, .. } => {
                children.insert(filename.to_string(), VfsNode::new_file(content));
                Ok(())
            }
            _ => Err(AgaveError::FileSystemError(FsError::NotDirectory)),
        }
    }

    /// Read entire file content
    pub fn read_file(&self, path: &str) -> AgaveResult<Vec<u8>> {
        let node = self.get_node(path)?;

        match node {
            VfsNode::File { content, .. } => Ok(content.clone()),
            VfsNode::Directory { .. } => Err(AgaveError::FileSystemError(FsError::IsDirectory)),
            VfsNode::Symlink { target, .. } => self.read_file(target), // Follow symlink
        }
    }

    /// Check if path exists
    pub fn exists(&self, path: &str) -> bool {
        self.get_node(path).is_ok()
    }

    /// Check if path is a directory
    pub fn is_dir(&self, path: &str) -> bool {
        if let Ok(node) = self.get_node(path) {
            matches!(node.metadata().file_type, FileType::Directory)
        } else {
            false
        }
    }

    /// Check if path is a file
    pub fn is_file(&self, path: &str) -> bool {
        if let Ok(node) = self.get_node(path) {
            matches!(node.metadata().file_type, FileType::Regular)
        } else {
            false
        }
    }

    // Internal helper methods
    fn get_node(&self, path: &str) -> AgaveResult<&VfsNode> {
        let parts: Vec<&str> = path
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        let mut current = &self.root;

        for part in parts {
            match current {
                VfsNode::Directory { children, .. } => {
                    current = children.get(part).ok_or(AgaveError::NotFound)?;
                }
                VfsNode::Symlink { target, .. } => {
                    // Follow symlink
                    return self.get_node(target);
                }
                _ => return Err(AgaveError::FileSystemError(FsError::NotDirectory)),
            }
        }

        Ok(current)
    }

    fn get_node_mut(&mut self, path: &str) -> AgaveResult<&mut VfsNode> {
        let parts: Vec<&str> = path
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        let mut current = &mut self.root;

        for part in parts {
            match current {
                VfsNode::Directory { children, .. } => {
                    current = children.get_mut(part).ok_or(AgaveError::NotFound)?;
                }
                _ => return Err(AgaveError::FileSystemError(FsError::NotDirectory)),
            }
        }

        Ok(current)
    }
}

/// Global file system instance
static mut FILESYSTEM: Option<Mutex<VirtualFileSystem>> = None;

/// Public API functions
pub fn init_filesystem() -> AgaveResult<()> {
    log::info!("Initializing virtual file system...");

    unsafe {
        FILESYSTEM = Some(Mutex::new(VirtualFileSystem::new()));
    }

    log::info!("Virtual file system initialized");
    Ok(())
}

/// Helper to access the filesystem safely
fn with_filesystem<F, R>(f: F) -> AgaveResult<R>
where
    F: FnOnce(&mut VirtualFileSystem) -> AgaveResult<R>,
{
    unsafe {
        if let Some(fs) = &FILESYSTEM {
            let mut guard = fs.lock();
            f(&mut *guard)
        } else {
            Err(AgaveError::NotReady)
        }
    }
}

pub fn open(path: &str, readable: bool, writable: bool) -> AgaveResult<u64> {
    with_filesystem(|fs| fs.open(path, readable, writable))
}

pub fn close(fd: u64) -> AgaveResult<()> {
    with_filesystem(|fs| fs.close(fd))
}

pub fn read(fd: u64, buffer: &mut [u8]) -> AgaveResult<usize> {
    with_filesystem(|fs| fs.read(fd, buffer))
}

pub fn write(fd: u64, data: &[u8]) -> AgaveResult<usize> {
    with_filesystem(|fs| fs.write(fd, data))
}

pub fn read_dir(path: &str) -> AgaveResult<Vec<DirEntry>> {
    with_filesystem(|fs| fs.read_dir(path))
}

pub fn metadata(path: &str) -> AgaveResult<FileMetadata> {
    with_filesystem(|fs| fs.metadata(path))
}

pub fn create_dir(path: &str) -> AgaveResult<()> {
    with_filesystem(|fs| fs.create_dir(path))
}

pub fn create_dir_all(path: &str) -> AgaveResult<()> {
    with_filesystem(|fs| fs.create_dir_all(path))
}

pub fn remove(path: &str) -> AgaveResult<()> {
    with_filesystem(|fs| fs.remove(path))
}

pub fn write_file(path: &str, content: Vec<u8>) -> AgaveResult<()> {
    with_filesystem(|fs| fs.write_file(path, content))
}

pub fn read_file(path: &str) -> AgaveResult<Vec<u8>> {
    with_filesystem(|fs| fs.read_file(path))
}

pub fn exists(path: &str) -> bool {
    with_filesystem(|fs| Ok(fs.exists(path))).unwrap_or(false)
}

pub fn is_dir(path: &str) -> bool {
    with_filesystem(|fs| Ok(fs.is_dir(path))).unwrap_or(false)
}

pub fn is_file(path: &str) -> bool {
    with_filesystem(|fs| Ok(fs.is_file(path))).unwrap_or(false)
}

/// Helper functions
fn get_parent_path(path: &str) -> String {
    if path == "/" {
        return "/".to_string();
    }

    let path = path.trim_end_matches('/');
    if let Some(pos) = path.rfind('/') {
        if pos == 0 {
            "/".to_string()
        } else {
            path[..pos].to_string()
        }
    } else {
        "/".to_string()
    }
}

fn get_filename(path: &str) -> &str {
    let path = path.trim_end_matches('/');
    if let Some(pos) = path.rfind('/') {
        &path[pos + 1..]
    } else {
        path
    }
}
