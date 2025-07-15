/// Pipe implementation for IPC
use crate::sys::{
    error::{AgaveError, AgaveResult},
    ipc::{IpcPermissions, ProcessId},
};
use alloc::{collections::VecDeque, sync::Arc};
use spin::Mutex;

/// Pipe buffer size constants
pub const DEFAULT_PIPE_BUFFER_SIZE: usize = 4096;
pub const MAX_PIPE_BUFFER_SIZE: usize = 64 * 1024; // 64KB

/// Pipe end types
#[derive(Debug, Clone)]
pub struct PipeEnd {
    buffer: Arc<Mutex<VecDeque<u8>>>,
    is_closed: Arc<Mutex<bool>>,
    capacity: usize,
}

impl PipeEnd {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            is_closed: Arc::new(Mutex::new(false)),
            capacity,
        }
    }

    fn is_closed(&self) -> bool {
        *self.is_closed.lock()
    }

    fn close(&self) {
        *self.is_closed.lock() = true;
    }

    fn available_space(&self) -> usize {
        let buffer = self.buffer.lock();
        self.capacity.saturating_sub(buffer.len())
    }

    fn available_data(&self) -> usize {
        self.buffer.lock().len()
    }
}

/// Pipe structure representing a unidirectional communication channel
#[derive(Debug, Clone)]
pub struct Pipe {
    pub read_end: Option<PipeEnd>,
    pub write_end: Option<PipeEnd>,
    pub permissions: IpcPermissions,
    pub owner: ProcessId,
    pub buffer_size: usize,
}

impl Pipe {
    /// Create a new pipe with default buffer size
    pub fn new() -> AgaveResult<Self> {
        Self::with_capacity(DEFAULT_PIPE_BUFFER_SIZE)
    }

    /// Create a new pipe with specified buffer capacity
    pub fn with_capacity(capacity: usize) -> AgaveResult<Self> {
        if capacity == 0 || capacity > MAX_PIPE_BUFFER_SIZE {
            return Err(AgaveError::InvalidParameter);
        }

        let pipe_end = PipeEnd::new(capacity);

        Ok(Self {
            read_end: Some(pipe_end.clone()),
            write_end: Some(pipe_end),
            permissions: IpcPermissions::default(),
            owner: 0, // Will be set by the IPC manager
            buffer_size: capacity,
        })
    }

    /// Read data from the pipe
    pub fn read(&mut self, buffer: &mut [u8]) -> AgaveResult<usize> {
        let read_end = self.read_end.as_ref().ok_or(AgaveError::InvalidOperation)?;

        if read_end.is_closed() {
            return Ok(0); // EOF
        }

        let mut pipe_buffer = read_end.buffer.lock();
        let bytes_to_read = buffer.len().min(pipe_buffer.len());

        if bytes_to_read == 0 {
            // No data available, check if write end is closed
            return if self.write_end.as_ref().map_or(true, |w| w.is_closed()) {
                Ok(0) // EOF - write end closed
            } else {
                Err(AgaveError::WouldBlock) // Would block - no data but write end open
            };
        }

        // Copy data from pipe buffer to user buffer
        for i in 0..bytes_to_read {
            buffer[i] = pipe_buffer.pop_front().unwrap();
        }

        log::trace!("Pipe read: {} bytes", bytes_to_read);
        Ok(bytes_to_read)
    }

    /// Write data to the pipe
    pub fn write(&mut self, data: &[u8]) -> AgaveResult<usize> {
        let write_end = self
            .write_end
            .as_ref()
            .ok_or(AgaveError::InvalidOperation)?;

        if write_end.is_closed() {
            return Err(AgaveError::BrokenPipe);
        }

        // Check if read end is closed
        if self.read_end.as_ref().map_or(true, |r| r.is_closed()) {
            return Err(AgaveError::BrokenPipe);
        }

        let mut pipe_buffer = write_end.buffer.lock();
        let available_space = write_end.capacity.saturating_sub(pipe_buffer.len());

        if available_space == 0 {
            return Err(AgaveError::WouldBlock); // Pipe buffer full
        }

        let bytes_to_write = data.len().min(available_space);

        // Copy data from user buffer to pipe buffer
        for &byte in &data[..bytes_to_write] {
            pipe_buffer.push_back(byte);
        }

        log::trace!("Pipe write: {} bytes", bytes_to_write);
        Ok(bytes_to_write)
    }

    /// Close the read end of the pipe
    pub fn close_read(&mut self) {
        if let Some(read_end) = &self.read_end {
            read_end.close();
        }
        self.read_end = None;
    }

    /// Close the write end of the pipe
    pub fn close_write(&mut self) {
        if let Some(write_end) = &self.write_end {
            write_end.close();
        }
        self.write_end = None;
    }

    /// Check if pipe is readable (has data or write end closed)
    pub fn is_readable(&self) -> bool {
        if let Some(read_end) = &self.read_end {
            !read_end.is_closed()
                && (read_end.available_data() > 0
                    || self.write_end.as_ref().map_or(true, |w| w.is_closed()))
        } else {
            false
        }
    }

    /// Check if pipe is writable (has space and read end open)
    pub fn is_writable(&self) -> bool {
        if let Some(write_end) = &self.write_end {
            !write_end.is_closed()
                && write_end.available_space() > 0
                && self.read_end.as_ref().map_or(false, |r| !r.is_closed())
        } else {
            false
        }
    }

    /// Get pipe statistics
    pub fn get_stats(&self) -> PipeStats {
        let (buffer_used, buffer_capacity) = if let Some(end) = &self.read_end {
            (end.available_data(), end.capacity)
        } else if let Some(end) = &self.write_end {
            (end.available_data(), end.capacity)
        } else {
            (0, 0)
        };

        PipeStats {
            buffer_used,
            buffer_capacity,
            read_end_open: self.read_end.as_ref().map_or(false, |r| !r.is_closed()),
            write_end_open: self.write_end.as_ref().map_or(false, |w| !w.is_closed()),
            is_readable: self.is_readable(),
            is_writable: self.is_writable(),
        }
    }
}

/// Pipe statistics
#[derive(Debug, Clone)]
pub struct PipeStats {
    pub buffer_used: usize,
    pub buffer_capacity: usize,
    pub read_end_open: bool,
    pub write_end_open: bool,
    pub is_readable: bool,
    pub is_writable: bool,
}

/// Named pipe for filesystem-based IPC
#[derive(Debug)]
pub struct NamedPipe {
    pub name: alloc::string::String,
    pub pipe: Pipe,
    pub creation_time: u64,
    pub last_accessed: u64,
}

impl NamedPipe {
    pub fn new(name: alloc::string::String, capacity: usize) -> AgaveResult<Self> {
        let current_time =
            crate::sys::interrupts::TIME_MS.load(core::sync::atomic::Ordering::Relaxed);

        Ok(Self {
            name,
            pipe: Pipe::with_capacity(capacity)?,
            creation_time: current_time,
            last_accessed: current_time,
        })
    }

    pub fn update_access_time(&mut self) {
        self.last_accessed =
            crate::sys::interrupts::TIME_MS.load(core::sync::atomic::Ordering::Relaxed);
    }
}
