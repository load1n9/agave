// WASI Error handling for Agave OS
use super::types::*;
use alloc::format;
use alloc::string::String;

#[derive(Debug, Clone)]
pub struct WasiError {
    pub errno: Errno,
    pub message: String,
}

impl WasiError {
    pub fn new(errno: Errno, message: &str) -> Self {
        Self {
            errno,
            message: message.into(),
        }
    }

    pub fn success() -> Self {
        Self::new(ERRNO_SUCCESS, "Success")
    }

    pub fn badf() -> Self {
        Self::new(ERRNO_BADF, "Bad file descriptor")
    }

    pub fn inval() -> Self {
        Self::new(ERRNO_INVAL, "Invalid argument")
    }

    pub fn noent() -> Self {
        Self::new(ERRNO_NOENT, "No such file or directory")
    }

    pub fn nomem() -> Self {
        Self::new(ERRNO_NOMEM, "Out of memory")
    }

    pub fn nosys() -> Self {
        Self::new(ERRNO_NOSYS, "Function not implemented")
    }

    pub fn perm() -> Self {
        Self::new(ERRNO_PERM, "Operation not permitted")
    }

    pub fn io() -> Self {
        Self::new(ERRNO_IO, "I/O error")
    }

    pub fn notdir() -> Self {
        Self::new(ERRNO_NOTDIR, "Not a directory")
    }

    pub fn isdir() -> Self {
        Self::new(ERRNO_ISDIR, "Is a directory")
    }

    pub fn exist() -> Self {
        Self::new(ERRNO_EXIST, "File exists")
    }

    pub fn notcapable() -> Self {
        Self::new(ERRNO_NOTCAPABLE, "Insufficient capabilities")
    }

    pub fn spipe() -> Self {
        Self::new(ERRNO_SPIPE, "Invalid seek")
    }

    pub fn fbig() -> Self {
        Self::new(ERRNO_FBIG, "File too large")
    }

    pub fn nospc() -> Self {
        Self::new(ERRNO_NOSPC, "No space left on device")
    }

    pub fn rofs() -> Self {
        Self::new(ERRNO_ROFS, "Read-only file system")
    }

    pub fn mlink() -> Self {
        Self::new(ERRNO_MLINK, "Too many links")
    }

    pub fn nametoolong() -> Self {
        Self::new(ERRNO_NAMETOOLONG, "File name too long")
    }

    pub fn notempty() -> Self {
        Self::new(ERRNO_NOTEMPTY, "Directory not empty")
    }

    pub fn loop_() -> Self {
        Self::new(ERRNO_LOOP, "Too many symbolic links encountered")
    }

    pub fn notconn() -> Self {
        Self::new(ERRNO_NOTCONN, "Socket is not connected")
    }

    pub fn connrefused() -> Self {
        Self::new(ERRNO_CONNREFUSED, "Connection refused")
    }

    pub fn timedout() -> Self {
        Self::new(ERRNO_TIMEDOUT, "Connection timed out")
    }

    pub fn again() -> Self {
        Self::new(ERRNO_AGAIN, "Try again")
    }

    pub fn intr() -> Self {
        Self::new(ERRNO_INTR, "Interrupted system call")
    }

    pub fn pipe() -> Self {
        Self::new(ERRNO_PIPE, "Broken pipe")
    }

    pub fn connreset() -> Self {
        Self::new(ERRNO_CONNRESET, "Connection reset by peer")
    }

    pub fn connaborted() -> Self {
        Self::new(ERRNO_CONNABORTED, "Software caused connection abort")
    }

    pub fn netdown() -> Self {
        Self::new(ERRNO_NETDOWN, "Network is down")
    }

    pub fn netunreach() -> Self {
        Self::new(ERRNO_NETUNREACH, "Network is unreachable")
    }

    pub fn hostunreach() -> Self {
        Self::new(ERRNO_HOSTUNREACH, "No route to host")
    }

    pub fn notsup() -> Self {
        Self::new(ERRNO_NOTSUP, "Not supported")
    }

    pub fn already() -> Self {
        Self::new(ERRNO_ALREADY, "Connection already in progress")
    }

    pub fn to_debug_string(&self) -> String {
        format!("WasiError({}): {}", self.errno, self.message)
    }
}

pub type WasiResult<T> = Result<T, WasiError>;

// Convert to Preview 2 stream error
impl From<WasiError> for StreamError {
    fn from(error: WasiError) -> Self {
        match error.errno {
            ERRNO_SUCCESS => StreamError::Closed, // shouldn't happen
            _ => StreamError::LastOperationFailed(Error {
                message: error.message,
            }),
        }
    }
}

// Convert from Preview 2 stream error
impl From<StreamError> for WasiError {
    fn from(error: StreamError) -> Self {
        match error {
            StreamError::Closed => WasiError::new(ERRNO_IO, "Stream closed"),
            StreamError::LastOperationFailed(err) => WasiError::new(ERRNO_IO, &err.message),
        }
    }
}
