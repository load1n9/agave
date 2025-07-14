// WASI fundamental types and constants
use alloc::string::String;

// Preview 1 types (legacy)
pub type Errno = u16;
pub type Fd = u32;
pub type Size = u32;
pub type FileSize = u64;
pub type FileDelta = i64;
pub type Timestamp = u64;
pub type Clockid = u32;
pub type UserData = u64;
pub type ExitCode = u32;
pub type Signal = u8;
pub type RiFlags = u16;
pub type SiFlags = u16;
pub type SdFlags = u8;
pub type PreOpenType = u8;
pub type DirCookie = u64;
pub type FdFlags = u16;
pub type FdStat = [u8; 24];
pub type FileStat = [u8; 56];
pub type FstFlags = u16;
pub type LookupFlags = u32;
pub type OFlags = u16;
pub type Rights = u64;
pub type Whence = u8;
pub type Advice = u8;

// Error codes
pub const ERRNO_SUCCESS: Errno = 0;
pub const ERRNO_2BIG: Errno = 1;
pub const ERRNO_ACCES: Errno = 2;
pub const ERRNO_ADDRINUSE: Errno = 3;
pub const ERRNO_ADDRNOTAVAIL: Errno = 4;
pub const ERRNO_AFNOSUPPORT: Errno = 5;
pub const ERRNO_AGAIN: Errno = 6;
pub const ERRNO_ALREADY: Errno = 7;
pub const ERRNO_BADF: Errno = 8;
pub const ERRNO_BADMSG: Errno = 9;
pub const ERRNO_BUSY: Errno = 10;
pub const ERRNO_CANCELED: Errno = 11;
pub const ERRNO_CHILD: Errno = 12;
pub const ERRNO_CONNABORTED: Errno = 13;
pub const ERRNO_CONNREFUSED: Errno = 14;
pub const ERRNO_CONNRESET: Errno = 15;
pub const ERRNO_DEADLK: Errno = 16;
pub const ERRNO_DESTADDRREQ: Errno = 17;
pub const ERRNO_DOM: Errno = 18;
pub const ERRNO_DQUOT: Errno = 19;
pub const ERRNO_EXIST: Errno = 20;
pub const ERRNO_FAULT: Errno = 21;
pub const ERRNO_FBIG: Errno = 22;
pub const ERRNO_HOSTUNREACH: Errno = 23;
pub const ERRNO_IDRM: Errno = 24;
pub const ERRNO_ILSEQ: Errno = 25;
pub const ERRNO_INPROGRESS: Errno = 26;
pub const ERRNO_INTR: Errno = 27;
pub const ERRNO_INVAL: Errno = 28;
pub const ERRNO_IO: Errno = 29;
pub const ERRNO_ISCONN: Errno = 30;
pub const ERRNO_ISDIR: Errno = 31;
pub const ERRNO_LOOP: Errno = 32;
pub const ERRNO_MFILE: Errno = 33;
pub const ERRNO_MLINK: Errno = 34;
pub const ERRNO_MSGSIZE: Errno = 35;
pub const ERRNO_MULTIHOP: Errno = 36;
pub const ERRNO_NAMETOOLONG: Errno = 37;
pub const ERRNO_NETDOWN: Errno = 38;
pub const ERRNO_NETRESET: Errno = 39;
pub const ERRNO_NETUNREACH: Errno = 40;
pub const ERRNO_NFILE: Errno = 41;
pub const ERRNO_NOBUFS: Errno = 42;
pub const ERRNO_NODEV: Errno = 43;
pub const ERRNO_NOENT: Errno = 44;
pub const ERRNO_NOEXEC: Errno = 45;
pub const ERRNO_NOLCK: Errno = 46;
pub const ERRNO_NOLINK: Errno = 47;
pub const ERRNO_NOMEM: Errno = 48;
pub const ERRNO_NOMSG: Errno = 49;
pub const ERRNO_NOPROTOOPT: Errno = 50;
pub const ERRNO_NOSPC: Errno = 51;
pub const ERRNO_NOSYS: Errno = 52;
pub const ERRNO_NOTCONN: Errno = 53;
pub const ERRNO_NOTDIR: Errno = 54;
pub const ERRNO_NOTEMPTY: Errno = 55;
pub const ERRNO_NOTRECOVERABLE: Errno = 56;
pub const ERRNO_NOTSOCK: Errno = 57;
pub const ERRNO_NOTSUP: Errno = 58;
pub const ERRNO_NOTTY: Errno = 59;
pub const ERRNO_NXIO: Errno = 60;
pub const ERRNO_OVERFLOW: Errno = 61;
pub const ERRNO_OWNERDEAD: Errno = 62;
pub const ERRNO_PERM: Errno = 63;
pub const ERRNO_PIPE: Errno = 64;
pub const ERRNO_PROTO: Errno = 65;
pub const ERRNO_PROTONOSUPPORT: Errno = 66;
pub const ERRNO_PROTOTYPE: Errno = 67;
pub const ERRNO_RANGE: Errno = 68;
pub const ERRNO_ROFS: Errno = 69;
pub const ERRNO_SPIPE: Errno = 70;
pub const ERRNO_SRCH: Errno = 71;
pub const ERRNO_STALE: Errno = 72;
pub const ERRNO_TIMEDOUT: Errno = 73;
pub const ERRNO_TXTBSY: Errno = 74;
pub const ERRNO_XDEV: Errno = 75;
pub const ERRNO_NOTCAPABLE: Errno = 76;

// Clock IDs
pub const CLOCKID_REALTIME: Clockid = 0;
pub const CLOCKID_MONOTONIC: Clockid = 1;
pub const CLOCKID_PROCESS_CPUTIME_ID: Clockid = 2;
pub const CLOCKID_THREAD_CPUTIME_ID: Clockid = 3;

// File descriptor flags
pub const FDFLAGS_APPEND: FdFlags = 1;
pub const FDFLAGS_DSYNC: FdFlags = 2;
pub const FDFLAGS_NONBLOCK: FdFlags = 4;
pub const FDFLAGS_RSYNC: FdFlags = 8;
pub const FDFLAGS_SYNC: FdFlags = 16;

// File types
pub const FILETYPE_UNKNOWN: u8 = 0;
pub const FILETYPE_BLOCK_DEVICE: u8 = 1;
pub const FILETYPE_CHARACTER_DEVICE: u8 = 2;
pub const FILETYPE_DIRECTORY: u8 = 3;
pub const FILETYPE_REGULAR_FILE: u8 = 4;
pub const FILETYPE_SOCKET_DGRAM: u8 = 5;
pub const FILETYPE_SOCKET_STREAM: u8 = 6;
pub const FILETYPE_SYMBOLIC_LINK: u8 = 7;

// Rights
pub const RIGHTS_FD_DATASYNC: Rights = 1;
pub const RIGHTS_FD_READ: Rights = 2;
pub const RIGHTS_FD_SEEK: Rights = 4;
pub const RIGHTS_FD_FDSTAT_SET_FLAGS: Rights = 8;
pub const RIGHTS_FD_SYNC: Rights = 16;
pub const RIGHTS_FD_TELL: Rights = 32;
pub const RIGHTS_FD_WRITE: Rights = 64;
pub const RIGHTS_FD_ADVISE: Rights = 128;
pub const RIGHTS_FD_ALLOCATE: Rights = 256;
pub const RIGHTS_PATH_CREATE_DIRECTORY: Rights = 512;
pub const RIGHTS_PATH_CREATE_FILE: Rights = 1024;
pub const RIGHTS_PATH_LINK_SOURCE: Rights = 2048;
pub const RIGHTS_PATH_LINK_TARGET: Rights = 4096;
pub const RIGHTS_PATH_OPEN: Rights = 8192;
pub const RIGHTS_FD_READDIR: Rights = 16384;
pub const RIGHTS_PATH_READLINK: Rights = 32768;
pub const RIGHTS_PATH_RENAME_SOURCE: Rights = 65536;
pub const RIGHTS_PATH_RENAME_TARGET: Rights = 131072;
pub const RIGHTS_PATH_FILESTAT_GET: Rights = 262144;
pub const RIGHTS_PATH_FILESTAT_SET_SIZE: Rights = 524288;
pub const RIGHTS_PATH_FILESTAT_SET_TIMES: Rights = 1048576;
pub const RIGHTS_FD_FILESTAT_GET: Rights = 2097152;
pub const RIGHTS_FD_FILESTAT_SET_SIZE: Rights = 4194304;
pub const RIGHTS_FD_FILESTAT_SET_TIMES: Rights = 8388608;
pub const RIGHTS_PATH_SYMLINK: Rights = 16777216;
pub const RIGHTS_PATH_REMOVE_DIRECTORY: Rights = 33554432;
pub const RIGHTS_PATH_UNLINK_FILE: Rights = 67108864;
pub const RIGHTS_POLL_FD_READWRITE: Rights = 134217728;
pub const RIGHTS_SOCK_SHUTDOWN: Rights = 268435456;

// Preview 2 types (component model)
#[derive(Debug, Clone)]
pub struct Error {
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum StreamError {
    LastOperationFailed(Error),
    Closed,
}

pub type InputStream = u32;
pub type OutputStream = u32;
pub type Pollable = u32;

// IOVec structure for scatter-gather I/O
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IOVec {
    pub buf: u32,     // pointer to buffer
    pub buf_len: u32, // length of buffer
}

// CIOVec structure for const scatter-gather I/O
pub type CIOVec = IOVec;

// Subscription for polling
#[repr(C)]
pub struct Subscription {
    pub userdata: UserData,
    pub u: SubscriptionU,
}

#[repr(C)]
pub union SubscriptionU {
    pub tag: u8,
    pub clock: SubscriptionClock,
    pub fd_read: SubscriptionFdReadwrite,
    pub fd_write: SubscriptionFdReadwrite,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SubscriptionClock {
    pub tag: u8,
    pub id: Clockid,
    pub timeout: Timestamp,
    pub precision: Timestamp,
    pub flags: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SubscriptionFdReadwrite {
    pub tag: u8,
    pub fd: Fd,
}

// Event structure for polling results
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub userdata: UserData,
    pub error: Errno,
    pub type_: u8,
    pub fd_readwrite: EventFdReadwrite,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct EventFdReadwrite {
    pub nbytes: FileSize,
    pub flags: u16,
}

// Prestat structure for preopened directories
#[repr(C)]
pub struct Prestat {
    pub tag: PreOpenType,
    pub u: PrestatU,
}

#[repr(C)]
pub union PrestatU {
    pub dir: PrestatDir,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PrestatDir {
    pub pr_name_len: Size,
}

// Directory entry
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Dirent {
    pub d_next: DirCookie,
    pub d_ino: u64,
    pub d_namlen: Size,
    pub d_type: u8,
}
