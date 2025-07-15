/// Simple file system with persistence support
use crate::sys::{
    error::{AgaveError, AgaveResult},
    fs::disk::{BlockNumber, DiskBackend, BLOCK_SIZE},
};
use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::mem;

/// Simple file system magic number
const FS_MAGIC: u32 = 0x41474156; // "AGAV" in ASCII

/// File system version
const FS_VERSION: u32 = 1;

/// Size of on-disk structures
const SUPERBLOCK_SIZE: usize = 512;
const INODE_SIZE: usize = 128;
const DIR_ENTRY_SIZE: usize = 64;

/// Maximum filename length
const MAX_FILENAME_LEN: usize = 48;

/// Inode number type
pub type InodeNumber = u32;

/// Special inode numbers
pub const ROOT_INODE: InodeNumber = 1;
pub const INVALID_INODE: InodeNumber = 0;

/// On-disk superblock structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Superblock {
    pub magic: u32,
    pub version: u32,
    pub block_size: u32,
    pub total_blocks: u32,
    pub free_blocks: u32,
    pub total_inodes: u32,
    pub free_inodes: u32,
    pub first_data_block: u32,
    pub inode_table_block: u32,
    pub free_block_bitmap_block: u32,
    pub free_inode_bitmap_block: u32,
    pub root_inode: u32,
    pub last_mount_time: u64,
    pub last_check_time: u64,
    pub mount_count: u32,
    pub max_mount_count: u32,
    pub state: u32,          // 0 = clean, 1 = errors
    pub reserved: [u32; 32], // For future use
}

impl Superblock {
    pub fn new(total_blocks: u32) -> Self {
        let inode_count = total_blocks / 8; // 1/8 of blocks for inodes
        let bitmap_blocks = 2; // One for block bitmap, one for inode bitmap
        let inode_table_blocks =
            (inode_count * INODE_SIZE as u32 + BLOCK_SIZE as u32 - 1) / BLOCK_SIZE as u32;

        Self {
            magic: FS_MAGIC,
            version: FS_VERSION,
            block_size: BLOCK_SIZE as u32,
            total_blocks,
            free_blocks: total_blocks - 1 - bitmap_blocks - inode_table_blocks, // -1 for superblock
            total_inodes: inode_count,
            free_inodes: inode_count - 1, // -1 for root inode
            first_data_block: 1 + bitmap_blocks + inode_table_blocks,
            inode_table_block: 1 + bitmap_blocks,
            free_block_bitmap_block: 1,
            free_inode_bitmap_block: 2,
            root_inode: ROOT_INODE,
            last_mount_time: 0,
            last_check_time: 0,
            mount_count: 0,
            max_mount_count: 50,
            state: 0,
            reserved: [0; 32],
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == FS_MAGIC && self.version == FS_VERSION && self.block_size == BLOCK_SIZE as u32
    }
}

/// File types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FileType {
    Regular = 1,
    Directory = 2,
    Symlink = 3,
    BlockDevice = 4,
    CharDevice = 5,
    Fifo = 6,
    Socket = 7,
}

impl FileType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(FileType::Regular),
            2 => Some(FileType::Directory),
            3 => Some(FileType::Symlink),
            4 => Some(FileType::BlockDevice),
            5 => Some(FileType::CharDevice),
            6 => Some(FileType::Fifo),
            7 => Some(FileType::Socket),
            _ => None,
        }
    }
}

/// On-disk inode structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Inode {
    pub file_type: u8,
    pub permissions: u16,
    pub link_count: u16,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub blocks: u32,
    pub atime: u64,                 // Access time
    pub mtime: u64,                 // Modification time
    pub ctime: u64,                 // Creation time
    pub direct_blocks: [u32; 12],   // Direct block pointers
    pub indirect_block: u32,        // Single indirect block
    pub double_indirect_block: u32, // Double indirect block
    pub triple_indirect_block: u32, // Triple indirect block
}

impl Default for Inode {
    fn default() -> Self {
        Self {
            file_type: 0,
            permissions: 0o644,
            link_count: 0,
            uid: 0,
            gid: 0,
            size: 0,
            blocks: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            direct_blocks: [0; 12],
            indirect_block: 0,
            double_indirect_block: 0,
            triple_indirect_block: 0,
        }
    }
}

impl Inode {
    pub fn new_file(permissions: u16, uid: u32, gid: u32) -> Self {
        let now = crate::sys::interrupts::TIME_MS.load(core::sync::atomic::Ordering::Relaxed);
        Self {
            file_type: FileType::Regular as u8,
            permissions,
            link_count: 1,
            uid,
            gid,
            atime: now,
            mtime: now,
            ctime: now,
            ..Default::default()
        }
    }

    pub fn new_directory(permissions: u16, uid: u32, gid: u32) -> Self {
        let now = crate::sys::interrupts::TIME_MS.load(core::sync::atomic::Ordering::Relaxed);
        Self {
            file_type: FileType::Directory as u8,
            permissions: permissions | 0o111, // Ensure execute bit for directories
            link_count: 2,                    // . and .. entries
            uid,
            gid,
            atime: now,
            mtime: now,
            ctime: now,
            ..Default::default()
        }
    }

    pub fn get_file_type(&self) -> Option<FileType> {
        FileType::from_u8(self.file_type)
    }

    pub fn is_directory(&self) -> bool {
        self.file_type == FileType::Directory as u8
    }

    pub fn is_regular_file(&self) -> bool {
        self.file_type == FileType::Regular as u8
    }
}

/// Directory entry structure
#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    pub inode: u32,
    pub name_len: u8,
    pub file_type: u8,
    pub name: [u8; MAX_FILENAME_LEN],
    pub reserved: [u8; 6],
}

impl DirectoryEntry {
    pub fn new(inode: InodeNumber, name: &str, file_type: FileType) -> AgaveResult<Self> {
        if name.len() > MAX_FILENAME_LEN {
            return Err(AgaveError::InvalidParameter);
        }

        let mut entry = Self {
            inode,
            name_len: name.len() as u8,
            file_type: file_type as u8,
            name: [0; MAX_FILENAME_LEN],
            reserved: [0; 6],
        };

        entry.name[..name.len()].copy_from_slice(name.as_bytes());
        Ok(entry)
    }

    pub fn get_name(&self) -> String {
        String::from_utf8_lossy(&self.name[..self.name_len as usize]).to_string()
    }

    pub fn get_file_type(&self) -> Option<FileType> {
        FileType::from_u8(self.file_type)
    }
}

/// Simple file system implementation
pub struct SimpleFileSystem<D: DiskBackend> {
    disk: D,
    superblock: Superblock,
    block_bitmap: Vec<u8>,
    inode_bitmap: Vec<u8>,
    inode_cache: BTreeMap<InodeNumber, Inode>,
    dirty_inodes: BTreeMap<InodeNumber, bool>,
    mounted: bool,
}

impl<D: DiskBackend> SimpleFileSystem<D> {
    /// Create a new file system on the disk
    pub fn format(mut disk: D) -> AgaveResult<Self> {
        if !disk.is_writable() {
            return Err(AgaveError::PermissionDenied);
        }

        let total_blocks = disk.total_blocks() as u32;
        let superblock = Superblock::new(total_blocks);
        let sb_total_inodes = superblock.total_inodes;

        log::info!(
            "Formatting file system: {} blocks, {} inodes",
            total_blocks,
            sb_total_inodes
        );

        // Write superblock
        let mut block = [0u8; BLOCK_SIZE];
        unsafe {
            let sb_bytes = core::slice::from_raw_parts(
                &superblock as *const _ as *const u8,
                mem::size_of::<Superblock>(),
            );
            block[..sb_bytes.len()].copy_from_slice(sb_bytes);
        }
        disk.write_block(0, &block)?;

        // Initialize block bitmap (all blocks initially free except system blocks)
        let bitmap_size = (total_blocks + 7) / 8;
        let mut block_bitmap = vec![0u8; bitmap_size as usize];

        // Mark system blocks as used
        for i in 0..superblock.first_data_block {
            let byte_index = (i / 8) as usize;
            let bit_index = i % 8;
            block_bitmap[byte_index] |= 1 << bit_index;
        }

        // Write block bitmap
        let mut bitmap_block = [0u8; BLOCK_SIZE];
        let copy_len = bitmap_size.min(BLOCK_SIZE as u32) as usize;
        bitmap_block[..copy_len].copy_from_slice(&block_bitmap[..copy_len]);
        disk.write_block(superblock.free_block_bitmap_block as u64, &bitmap_block)?;

        // Initialize inode bitmap (all inodes free except root)
        let inode_bitmap_size = (superblock.total_inodes + 7) / 8;
        let mut inode_bitmap = vec![0u8; inode_bitmap_size as usize];

        // Mark root inode as used
        let byte_index = ((ROOT_INODE - 1) / 8) as usize;
        let bit_index = (ROOT_INODE - 1) % 8;
        if byte_index < inode_bitmap.len() {
            inode_bitmap[byte_index] |= 1 << bit_index;
        }

        // Write inode bitmap
        let mut inode_bitmap_block = [0u8; BLOCK_SIZE];
        let copy_len = inode_bitmap_size.min(BLOCK_SIZE as u32) as usize;
        inode_bitmap_block[..copy_len].copy_from_slice(&inode_bitmap[..copy_len]);
        disk.write_block(
            superblock.free_inode_bitmap_block as u64,
            &inode_bitmap_block,
        )?;

        // Create root directory inode
        let root_inode = Inode::new_directory(0o755, 0, 0);

        // Write root inode to inode table
        let mut inode_block = [0u8; BLOCK_SIZE];
        let inode_offset =
            ((ROOT_INODE - 1) % (BLOCK_SIZE as u32 / INODE_SIZE as u32)) * INODE_SIZE as u32;
        unsafe {
            let inode_bytes =
                core::slice::from_raw_parts(&root_inode as *const _ as *const u8, INODE_SIZE);
            inode_block[inode_offset as usize..inode_offset as usize + INODE_SIZE]
                .copy_from_slice(inode_bytes);
        }

        let inode_table_block = superblock.inode_table_block
            + ((ROOT_INODE - 1) / (BLOCK_SIZE as u32 / INODE_SIZE as u32));
        disk.write_block(inode_table_block as u64, &inode_block)?;

        disk.flush()?;

        log::info!("File system formatted successfully");

        // Create the filesystem instance
        let mut fs = Self {
            disk,
            superblock,
            block_bitmap,
            inode_bitmap,
            inode_cache: BTreeMap::new(),
            dirty_inodes: BTreeMap::new(),
            mounted: false,
        };

        // Initialize root directory with . and .. entries
        fs.mounted = true; // Temporarily set for directory operations
        fs.inode_cache.insert(ROOT_INODE, root_inode);

        // Create . entry (current directory)
        let dot_entry = DirectoryEntry::new(ROOT_INODE, ".", FileType::Directory)?;
        fs.add_directory_entry(ROOT_INODE, &dot_entry)?;

        // Create .. entry (parent directory - same as current for root)
        let dotdot_entry = DirectoryEntry::new(ROOT_INODE, "..", FileType::Directory)?;
        fs.add_directory_entry(ROOT_INODE, &dotdot_entry)?;

        fs.sync()?;
        fs.mounted = false;

        Ok(fs)
    }

    /// Mount an existing file system
    pub fn mount(mut disk: D) -> AgaveResult<Self> {
        // Read superblock
        let mut block = [0u8; BLOCK_SIZE];
        disk.read_block(0, &mut block)?;

        let superblock = unsafe { core::ptr::read_unaligned(block.as_ptr() as *const Superblock) };

        if !superblock.is_valid() {
            return Err(AgaveError::FileSystemError(
                crate::sys::error::FsError::CorruptedData,
            ));
        }

        let sb_total_blocks = superblock.total_blocks;
        let sb_total_inodes = superblock.total_inodes;
        let sb_state = superblock.state;
        log::info!(
            "Mounting file system: {} blocks, {} inodes, state={}",
            sb_total_blocks,
            sb_total_inodes,
            sb_state
        );

        // Read block bitmap
        disk.read_block(superblock.free_block_bitmap_block as u64, &mut block)?;
        let bitmap_size = (superblock.total_blocks + 7) / 8;
        let mut block_bitmap = vec![0u8; bitmap_size as usize];
        let copy_len = bitmap_size.min(BLOCK_SIZE as u32) as usize;
        block_bitmap[..copy_len].copy_from_slice(&block[..copy_len]);

        // Read inode bitmap
        disk.read_block(superblock.free_inode_bitmap_block as u64, &mut block)?;
        let inode_bitmap_size = (superblock.total_inodes + 7) / 8;
        let mut inode_bitmap = vec![0u8; inode_bitmap_size as usize];
        let copy_len = inode_bitmap_size.min(BLOCK_SIZE as u32) as usize;
        inode_bitmap[..copy_len].copy_from_slice(&block[..copy_len]);

        let mut fs = Self {
            disk,
            superblock,
            block_bitmap,
            inode_bitmap,
            inode_cache: BTreeMap::new(),
            dirty_inodes: BTreeMap::new(),
            mounted: true,
        };

        // Load root inode
        let root_inode = fs.read_inode(ROOT_INODE)?;
        fs.inode_cache.insert(ROOT_INODE, root_inode);

        log::info!("File system mounted successfully");
        Ok(fs)
    }

    /// Unmount the file system
    pub fn unmount(&mut self) -> AgaveResult<()> {
        if !self.mounted {
            return Err(AgaveError::InvalidState);
        }

        self.sync()?;
        self.mounted = false;
        self.inode_cache.clear();
        self.dirty_inodes.clear();

        log::info!("File system unmounted");
        Ok(())
    }

    /// Sync all dirty data to disk
    pub fn sync(&mut self) -> AgaveResult<()> {
        if !self.mounted {
            return Err(AgaveError::InvalidState);
        }

        // Write dirty inodes
        let dirty_inodes: Vec<(u32, bool)> =
            self.dirty_inodes.iter().map(|(&k, &v)| (k, v)).collect();
        for (inode_num, is_dirty) in dirty_inodes {
            if is_dirty {
                if let Some(inode) = self.inode_cache.get(&inode_num).cloned() {
                    self.write_inode(inode_num, &inode)?;
                }
            }
        }
        self.dirty_inodes.clear();

        // Write bitmaps
        self.write_block_bitmap()?;
        self.write_inode_bitmap()?;

        // Write superblock
        self.write_superblock()?;

        self.disk.flush()?;

        log::debug!("File system synced to disk");
        Ok(())
    }

    /// Read an inode from disk
    fn read_inode(&mut self, inode_num: InodeNumber) -> AgaveResult<Inode> {
        if inode_num == 0 || inode_num > self.superblock.total_inodes {
            return Err(AgaveError::InvalidParameter);
        }

        // Check cache first
        if let Some(inode) = self.inode_cache.get(&inode_num) {
            return Ok(*inode);
        }

        // Calculate which block contains this inode
        let inodes_per_block = BLOCK_SIZE / INODE_SIZE;
        let block_num =
            self.superblock.inode_table_block + ((inode_num - 1) / inodes_per_block as u32);
        let inode_offset = ((inode_num - 1) % inodes_per_block as u32) * INODE_SIZE as u32;

        // Read the block
        let mut block = [0u8; BLOCK_SIZE];
        self.disk.read_block(block_num as u64, &mut block)?;

        // Extract the inode
        let inode = unsafe {
            core::ptr::read_unaligned((block.as_ptr().add(inode_offset as usize)) as *const Inode)
        };

        // Cache the inode
        self.inode_cache.insert(inode_num, inode);

        Ok(inode)
    }

    /// Write an inode to disk
    fn write_inode(&mut self, inode_num: InodeNumber, inode: &Inode) -> AgaveResult<()> {
        if inode_num == 0 || inode_num > self.superblock.total_inodes {
            return Err(AgaveError::InvalidParameter);
        }

        // Calculate which block contains this inode
        let inodes_per_block = BLOCK_SIZE / INODE_SIZE;
        let block_num =
            self.superblock.inode_table_block + ((inode_num - 1) / inodes_per_block as u32);
        let inode_offset = ((inode_num - 1) % inodes_per_block as u32) * INODE_SIZE as u32;

        // Read the block first
        let mut block = [0u8; BLOCK_SIZE];
        self.disk.read_block(block_num as u64, &mut block)?;

        // Update the inode in the block
        unsafe {
            let inode_bytes =
                core::slice::from_raw_parts(inode as *const _ as *const u8, INODE_SIZE);
            block[inode_offset as usize..inode_offset as usize + INODE_SIZE]
                .copy_from_slice(inode_bytes);
        }

        // Write the block back
        self.disk.write_block(block_num as u64, &block)?;

        // Update cache
        self.inode_cache.insert(inode_num, *inode);

        Ok(())
    }

    /// Add a directory entry
    fn add_directory_entry(
        &mut self,
        dir_inode: InodeNumber,
        entry: &DirectoryEntry,
    ) -> AgaveResult<()> {
        // This is a simplified implementation
        // In a real filesystem, you'd need to handle block allocation for directory data

        // For now, just mark the inode as dirty
        self.dirty_inodes.insert(dir_inode, true);

        log::debug!(
            "Added directory entry: {} to inode {}",
            entry.get_name(),
            dir_inode
        );
        Ok(())
    }

    /// Write superblock to disk
    fn write_superblock(&mut self) -> AgaveResult<()> {
        let mut block = [0u8; BLOCK_SIZE];
        unsafe {
            let sb_bytes = core::slice::from_raw_parts(
                &self.superblock as *const _ as *const u8,
                mem::size_of::<Superblock>(),
            );
            block[..sb_bytes.len()].copy_from_slice(sb_bytes);
        }
        self.disk.write_block(0, &block)
    }

    /// Write block bitmap to disk
    fn write_block_bitmap(&mut self) -> AgaveResult<()> {
        let mut block = [0u8; BLOCK_SIZE];
        let copy_len = self.block_bitmap.len().min(BLOCK_SIZE);
        block[..copy_len].copy_from_slice(&self.block_bitmap[..copy_len]);
        self.disk
            .write_block(self.superblock.free_block_bitmap_block as u64, &block)
    }

    /// Write inode bitmap to disk
    fn write_inode_bitmap(&mut self) -> AgaveResult<()> {
        let mut block = [0u8; BLOCK_SIZE];
        let copy_len = self.inode_bitmap.len().min(BLOCK_SIZE);
        block[..copy_len].copy_from_slice(&self.inode_bitmap[..copy_len]);
        self.disk
            .write_block(self.superblock.free_inode_bitmap_block as u64, &block)
    }

    /// Get file system statistics
    pub fn get_stats(&self) -> FileSystemStats {
        FileSystemStats {
            total_blocks: self.superblock.total_blocks as u64,
            free_blocks: self.superblock.free_blocks as u64,
            used_blocks: (self.superblock.total_blocks - self.superblock.free_blocks) as u64,
            total_inodes: self.superblock.total_inodes as u64,
            free_inodes: self.superblock.free_inodes as u64,
            used_inodes: (self.superblock.total_inodes - self.superblock.free_inodes) as u64,
            block_size: self.superblock.block_size as u64,
            mount_count: self.superblock.mount_count,
            last_mount_time: self.superblock.last_mount_time,
            fs_state: if self.superblock.state == 0 {
                "clean"
            } else {
                "dirty"
            }
            .to_string(),
            backend_type: self.disk.backend_type().to_string(),
        }
    }

    /// Check if the filesystem is mounted
    pub fn is_mounted(&self) -> bool {
        self.mounted
    }
}

/// File system statistics
#[derive(Debug, Clone)]
pub struct FileSystemStats {
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub used_blocks: u64,
    pub total_inodes: u64,
    pub free_inodes: u64,
    pub used_inodes: u64,
    pub block_size: u64,
    pub mount_count: u32,
    pub last_mount_time: u64,
    pub fs_state: String,
    pub backend_type: String,
}
