use crate::{api::io, sys::fs::FileType};
use alloc::string::String;
// use alloc::vec::Vec;
// use core::{cmp, convert::TryFrom as _};
// use core2::io::{Read, Write};
use hashbrown::HashMap;
pub const FIRST_NONRESERVED_FD: wasi::Fd = 3;

pub enum PosixNodeOrStdio {
    Stdin,
    Stdout,
    Stderr,
    Inode(PosixNode),
}

impl PosixNodeOrStdio {
    pub fn write(&mut self, buffer: &[u8]) -> Result<usize, wasi::Errno> {
        match self {
            PosixNodeOrStdio::Stdin => Err(wasi::ERRNO_NOTSUP),
            PosixNodeOrStdio::Stdout => match io::stdout().write_all(buffer) {
                Ok(_) => Ok(buffer.len()),
                Err(_) => Err(wasi::ERRNO_IO),
            },
            PosixNodeOrStdio::Stderr => match io::stderr().write_all(buffer) {
                Ok(_) => Ok(buffer.len()),
                Err(_) => Err(wasi::ERRNO_IO),
            },
            PosixNodeOrStdio::Inode(posix_node) => posix_node.write(buffer),
        }
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize, wasi::Errno> {
        match self {
            PosixNodeOrStdio::Stdin => match io::stdin().read(buffer) {
                Ok(bytes_read) => Ok(bytes_read),
                Err(_) => Err(wasi::ERRNO_IO),
            },
            PosixNodeOrStdio::Stdout => Err(wasi::ERRNO_NOTSUP),
            PosixNodeOrStdio::Stderr => Err(wasi::ERRNO_NOTSUP),
            PosixNodeOrStdio::Inode(posix_node) => posix_node.read(buffer),
        }
    }

    pub fn seek(
        &mut self,
        offset: wasi::Filedelta,
        whence: wasi::Whence,
    ) -> Result<usize, wasi::Errno> {
        match self {
            PosixNodeOrStdio::Stdin => Err(wasi::ERRNO_NOTSUP),
            PosixNodeOrStdio::Stdout => Err(wasi::ERRNO_NOTSUP),
            PosixNodeOrStdio::Stderr => Err(wasi::ERRNO_NOTSUP),
            PosixNodeOrStdio::Inode(posix_node) => posix_node.seek(offset, whence),
        }
    }
}

pub struct PosixNode {
    pub file_or_dir: FileType,
    fs_rights_base: wasi::Rights,
    fs_rights_inheriting: wasi::Rights,
    fs_flags: wasi::Fdflags,
    _offset: usize,
}

impl PosixNode {
    pub fn new(
        file_or_dir: FileType,
        fs_rights_base: wasi::Rights,
        fs_rights_inheriting: wasi::Rights,
        fs_flags: wasi::Fdflags,
    ) -> PosixNode {
        PosixNode {
            file_or_dir,
            fs_rights_base: fs_rights_base,
            fs_rights_inheriting: fs_rights_inheriting,
            fs_flags: fs_flags,
            _offset: 0,
        }
    }

    pub fn get_relative_path(&self) -> String {
        // let absolute_path = Path::new(self.file_or_dir.get_absolute_path());
        // let wd_path = Path::new(
        //     task::get_my_current_task()
        //         .unwrap()
        //         .get_env()
        //         .lock()
        //         .get_wd_path(),
        // );

        // let relative_path: Path = absolute_path.relative(&wd_path).unwrap();
        // String::from(relative_path)
        unimplemented!()
    }

    pub fn fs_rights_base(&self) -> wasi::Rights {
        self.fs_rights_base
    }

    pub fn fs_rights_inheriting(&self) -> wasi::Rights {
        self.fs_rights_inheriting
    }

    pub fn fs_flags(&self) -> wasi::Fdflags {
        self.fs_flags
    }

    pub fn set_fs_flags(&mut self, new_flags: wasi::Fdflags) -> Result<(), wasi::Errno> {
        // Verify that file descriptor has right to set flags.
        if self.fs_rights_base() & wasi::RIGHTS_FD_FDSTAT_SET_FLAGS == 0 {
            return Err(wasi::ERRNO_ACCES);
        }

        self.fs_flags = new_flags;
        Ok(())
    }

    pub fn write(&mut self, _buffer: &[u8]) -> Result<usize, wasi::Errno> {
        if self.fs_rights_base() & wasi::RIGHTS_FD_WRITE == 0 {
            return Err(wasi::ERRNO_ACCES);
        }

        match self.file_or_dir.clone() {
            // FileType::File(file_ref) => {
            //     let is_append_mode: bool = (self.fs_flags() & wasi::FDFLAGS_APPEND) != 0;

            //     if is_append_mode {
            //         let end_of_file_offset: usize = file_ref.lock().len();
            //         match file_ref.lock().write_at(buffer, end_of_file_offset) {
            //             Ok(bytes_written) => Ok(bytes_written),
            //             Err(_) => Err(wasi::ERRNO_NOBUFS),
            //         }
            //     } else {
            //         let offset = self.offset;
            //         match file_ref.lock().write_at(buffer, offset) {
            //             Ok(bytes_written) => {
            //                 self.offset = self.offset.checked_add(bytes_written).unwrap();
            //                 Ok(bytes_written)
            //             }
            //             Err(_) => Err(wasi::ERRNO_NOBUFS),
            //         }
            //     }
            // }
            FileType::Dir { .. } => Err(wasi::ERRNO_ISDIR),
            _ => {
                unimplemented!()
            }
        }
    }

    pub fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, wasi::Errno> {
        if self.fs_rights_base() & wasi::RIGHTS_FD_READ == 0 {
            return Err(wasi::ERRNO_ACCES);
        }

        match self.file_or_dir.clone() {
            // FileType::File(file_ref) => {
            //     let offset = self.offset;
            //     match file_ref.lock().read_at(buffer, offset) {
            //         Ok(bytes_read) => {
            //             self.offset = self.offset.checked_add(bytes_read).unwrap();
            //             Ok(bytes_read)
            //         }
            //         Err(_) => Err(wasi::ERRNO_NOBUFS),
            //     }
            // }
            FileType::Dir { .. } => Err(wasi::ERRNO_ISDIR),
            _ => {
                unimplemented!()
            }
        }
    }

    pub fn seek(
        &mut self,
        _delta: wasi::Filedelta,
        _whence: wasi::Whence,
    ) -> Result<usize, wasi::Errno> {
        if self.fs_rights_base() & wasi::RIGHTS_FD_SEEK == 0 {
            return Err(wasi::ERRNO_ACCES);
        }

        match self.file_or_dir.clone() {
            // FileType::File(file_ref) => {
            //     let max_offset: usize = file_ref.lock().len();

            //     let signed_to_file_offset = |x: i64| -> usize {
            //         cmp::min(usize::try_from(cmp::max(0, x)).unwrap(), max_offset)
            //     };

            //     let new_offset: usize = match whence {
            //         wasi::WHENCE_CUR => {
            //             signed_to_file_offset(i64::try_from(self.offset).unwrap() + delta)
            //         }
            //         wasi::WHENCE_END => {
            //             signed_to_file_offset(i64::try_from(max_offset).unwrap() + delta)
            //         }
            //         wasi::WHENCE_SET => signed_to_file_offset(delta),
            //         _ => {
            //             return Err(wasi::ERRNO_SPIPE);
            //         }
            //     };

            //     self.offset = new_offset;
            //     Ok(new_offset)
            // }
            FileType::Dir { .. } => Err(wasi::ERRNO_ISDIR),
            _ => {
                unimplemented!()
            }
        }
    }
}

pub struct FileDescriptorTable {
    fd_table: HashMap<wasi::Fd, PosixNodeOrStdio>,
}

impl FileDescriptorTable {
    pub fn new() -> FileDescriptorTable {
        let mut fd_table = HashMap::new();
        fd_table.insert(wasi::FD_STDIN, PosixNodeOrStdio::Stdin);
        fd_table.insert(wasi::FD_STDOUT, PosixNodeOrStdio::Stdout);
        fd_table.insert(wasi::FD_STDERR, PosixNodeOrStdio::Stderr);
        FileDescriptorTable { fd_table: fd_table }
    }

    // pub fn open_path(
    //     &mut self,
    //     path: &str,
    //     starting_dir: DirRef,
    //     lookup_flags: wasi::Lookupflags,
    //     open_flags: wasi::Oflags,
    //     fs_rights_base: wasi::Rights,
    //     fs_rights_inheriting: wasi::Rights,
    //     fs_flags: wasi::Fdflags,
    // ) -> Result<wasi::Fd, wasi::Errno> {
    //     let _symlink_follow: bool = (lookup_flags & wasi::LOOKUPFLAGS_SYMLINK_FOLLOW) != 0;

    //     let create_file_if_no_exist: bool = (open_flags & wasi::OFLAGS_CREAT) != 0;
    //     let fail_if_not_dir: bool = (open_flags & wasi::OFLAGS_DIRECTORY) != 0;
    //     let fail_if_file_exists: bool = (open_flags & wasi::OFLAGS_EXCL) != 0;
    //     let truncate_file_to_size_zero: bool = (open_flags & wasi::OFLAGS_TRUNC) != 0;

    //     let mut fd: wasi::Fd = FIRST_NONRESERVED_FD;
    //     while self.fd_table.contains_key(&fd) {
    //         fd += 1;
    //     }

    //     let file_path: Path = Path::new(String::from(path));
    //     let mut file_path_tokens: Vec<&str> = file_path.components().collect();
    //     file_path_tokens.truncate(file_path_tokens.len().saturating_sub(1));
    //     let parent_dir_path: Path = Path::new(file_path_tokens.join("/"));
    //     let base_name: &str = file_path.basename();
    //     let base_path: Path = Path::new(String::from(base_name));

    //     let parent_dir: DirRef = match parent_dir_path.get(&starting_dir) {
    //         Some(file_or_dir) => match file_or_dir {
    //             FileType::File { .. } => {
    //                 return Err(wasi::ERRNO_NOENT);
    //             }
    //             FileType::Dir(dir_ref) => dir_ref,
    //         },
    //         None => {
    //             return Err(wasi::ERRNO_NOENT);
    //         }
    //     };

    //     let opened_file_or_dir: FileType = match base_path.get(&parent_dir) {
    //         Some(file_or_dir) => match file_or_dir {
    //             FileType::File { .. } => {
    //                 if fail_if_file_exists {
    //                     return Err(wasi::ERRNO_EXIST);
    //                 } else if fail_if_not_dir {
    //                     return Err(wasi::ERRNO_NOTDIR);
    //                 } else {
    //                     if truncate_file_to_size_zero {
    //                         // HACK: Truncate file by overwriting file.
    //                         let new_file: FileRef =
    //                             MemFile::new(String::from(base_name), &parent_dir).unwrap();
    //                         FileType::File(new_file)
    //                     } else {
    //                         file_or_dir
    //                     }
    //                 }
    //             }
    //             FileType::Dir { .. } => file_or_dir,
    //         },
    //         None => {
    //             if create_file_if_no_exist {
    //                 let new_file: FileRef =
    //                     MemFile::new(String::from(base_name), &parent_dir).unwrap();
    //                 FileType::File(new_file)
    //             } else {
    //                 return Err(wasi::ERRNO_NOENT);
    //             }
    //         }
    //     };

    //     self.fd_table.insert(
    //         fd,
    //         PosixNodeOrStdio::Inode(PosixNode::new(
    //             opened_file_or_dir,
    //             fs_rights_base,
    //             fs_rights_inheriting,
    //             fs_flags,
    //         )),
    //     );

    //     Ok(fd)
    // }

    pub fn close_fd(&mut self, fd: wasi::Fd) -> Result<(), wasi::Errno> {
        if self.fd_table.contains_key(&fd)
            && fd != wasi::FD_STDIN
            && fd != wasi::FD_STDOUT
            && fd != wasi::FD_STDERR
        {
            self.fd_table.remove(&fd);
            return Ok(());
        }
        Err(wasi::ERRNO_BADF)
    }

    pub fn get_posix_node_or_stdio(&mut self, fd: wasi::Fd) -> Option<&mut PosixNodeOrStdio> {
        self.fd_table.get_mut(&fd)
    }

    pub fn get_posix_node(&mut self, fd: wasi::Fd) -> Option<&mut PosixNode> {
        match self.get_posix_node_or_stdio(fd) {
            Some(posix_node_or_stdio) => match posix_node_or_stdio {
                PosixNodeOrStdio::Inode(posix_node) => Some(posix_node),
                _ => None,
            },
            None => None,
        }
    }
}