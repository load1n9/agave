// WASI I/O streams implementation for Agave OS
use super::error::*;
use super::types::*;
pub use super::types::{InputStream, OutputStream, Pollable};
use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;
use spin::Mutex;

// Global stream registry
static STREAMS: Mutex<StreamRegistry> = Mutex::new(StreamRegistry::new());
pub static POLLABLES: Mutex<PollableRegistry> = Mutex::new(PollableRegistry::new());

#[derive(Debug)]
pub struct StreamRegistry {
    input_streams: BTreeMap<InputStream, InputStreamImpl>,
    output_streams: BTreeMap<OutputStream, OutputStreamImpl>,
    next_id: u32,
}

impl StreamRegistry {
    pub const fn new() -> Self {
        Self {
            input_streams: BTreeMap::new(),
            output_streams: BTreeMap::new(),
            next_id: 1,
        }
    }

    pub fn create_input_stream(&mut self, buffer: Vec<u8>) -> InputStream {
        let id = self.next_id;
        self.next_id += 1;
        self.input_streams.insert(id, InputStreamImpl::new(buffer));
        id
    }

    pub fn create_output_stream(&mut self) -> OutputStream {
        let id = self.next_id;
        self.next_id += 1;
        self.output_streams.insert(id, OutputStreamImpl::new());
        id
    }

    pub fn get_input_stream(&mut self, id: InputStream) -> Option<&mut InputStreamImpl> {
        self.input_streams.get_mut(&id)
    }

    pub fn get_output_stream(&mut self, id: OutputStream) -> Option<&mut OutputStreamImpl> {
        self.output_streams.get_mut(&id)
    }

    pub fn remove_input_stream(&mut self, id: InputStream) {
        self.input_streams.remove(&id);
    }

    pub fn remove_output_stream(&mut self, id: OutputStream) {
        self.output_streams.remove(&id);
    }
}

#[derive(Debug)]
pub struct PollableRegistry {
    pollables: BTreeMap<Pollable, PollableImpl>,
    next_id: u32,
}

impl PollableRegistry {
    pub const fn new() -> Self {
        Self {
            pollables: BTreeMap::new(),
            next_id: 1,
        }
    }

    pub fn create_pollable(&mut self, ready: bool) -> Pollable {
        let id = self.next_id;
        self.next_id += 1;
        self.pollables.insert(id, PollableImpl::new(ready));
        id
    }

    pub fn get_pollable(&mut self, id: Pollable) -> Option<&mut PollableImpl> {
        self.pollables.get_mut(&id)
    }

    pub fn remove_pollable(&mut self, id: Pollable) {
        self.pollables.remove(&id);
    }
}

#[derive(Debug)]
pub struct InputStreamImpl {
    buffer: Vec<u8>,
    position: usize,
    closed: bool,
}

impl InputStreamImpl {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            buffer,
            position: 0,
            closed: false,
        }
    }

    pub fn read(&mut self, len: u64) -> Result<Vec<u8>, StreamError> {
        if self.closed {
            return Err(StreamError::Closed);
        }

        let len = len.min(self.buffer.len().saturating_sub(self.position) as u64) as usize;
        let end = self.position + len;
        let data = self.buffer[self.position..end].to_vec();
        self.position = end;

        if self.position >= self.buffer.len() {
            self.closed = true;
        }

        Ok(data)
    }

    pub fn blocking_read(&mut self, len: u64) -> Result<Vec<u8>, StreamError> {
        // In a real implementation, this would block until data is available
        self.read(len)
    }

    pub fn skip(&mut self, len: u64) -> Result<u64, StreamError> {
        if self.closed {
            return Err(StreamError::Closed);
        }

        let len = len.min(self.buffer.len().saturating_sub(self.position) as u64);
        self.position += len as usize;

        if self.position >= self.buffer.len() {
            self.closed = true;
        }

        Ok(len)
    }

    pub fn blocking_skip(&mut self, len: u64) -> Result<u64, StreamError> {
        // In a real implementation, this would block until data is available
        self.skip(len)
    }

    pub fn subscribe(&self) -> Pollable {
        let mut pollables = POLLABLES.lock();
        pollables.create_pollable(!self.closed && self.position < self.buffer.len())
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }
}

#[derive(Debug)]
pub struct OutputStreamImpl {
    buffer: Vec<u8>,
    closed: bool,
    flushed: bool,
}

impl OutputStreamImpl {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            closed: false,
            flushed: true,
        }
    }

    pub fn check_write(&self) -> Result<u64, StreamError> {
        if self.closed {
            return Err(StreamError::Closed);
        }
        // Return a reasonable buffer size
        Ok(4096)
    }

    pub fn write(&mut self, contents: Vec<u8>) -> Result<(), StreamError> {
        if self.closed {
            return Err(StreamError::Closed);
        }

        self.buffer.extend_from_slice(&contents);
        self.flushed = false;
        Ok(())
    }

    pub fn blocking_write_and_flush(&mut self, contents: Vec<u8>) -> Result<(), StreamError> {
        self.write(contents)?;
        self.flush()
    }

    pub fn flush(&mut self) -> Result<(), StreamError> {
        if self.closed {
            return Err(StreamError::Closed);
        }

        // In a real implementation, this would flush to the actual output
        // For now, we'll just mark as flushed
        self.flushed = true;
        Ok(())
    }

    pub fn blocking_flush(&mut self) -> Result<(), StreamError> {
        // In a real implementation, this would block until flush completes
        self.flush()
    }

    pub fn subscribe(&self) -> Pollable {
        let mut pollables = POLLABLES.lock();
        pollables.create_pollable(!self.closed)
    }

    pub fn write_zeroes(&mut self, len: u64) -> Result<(), StreamError> {
        if self.closed {
            return Err(StreamError::Closed);
        }

        let zeroes = vec![0u8; len as usize];
        self.write(zeroes)
    }

    pub fn blocking_write_zeroes_and_flush(&mut self, len: u64) -> Result<(), StreamError> {
        self.write_zeroes(len)?;
        self.flush()
    }

    pub fn splice(&mut self, src: &mut InputStreamImpl, len: u64) -> Result<u64, StreamError> {
        let data = src.read(len)?;
        let bytes_transferred = data.len() as u64;
        self.write(data)?;
        Ok(bytes_transferred)
    }

    pub fn blocking_splice(
        &mut self,
        src: &mut InputStreamImpl,
        len: u64,
    ) -> Result<u64, StreamError> {
        // In a real implementation, this would block until ready
        self.splice(src, len)
    }

    pub fn get_buffer(&self) -> &[u8] {
        &self.buffer
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    pub fn close(&mut self) {
        self.closed = true;
    }
}

#[derive(Debug)]
pub struct PollableImpl {
    ready: bool,
}

impl PollableImpl {
    pub fn new(ready: bool) -> Self {
        Self { ready }
    }

    pub fn ready(&self) -> bool {
        self.ready
    }

    pub fn block(&mut self) {
        // In a real implementation, this would block until ready
        // For now, we'll just set ready to true
        self.ready = true;
    }

    pub fn set_ready(&mut self, ready: bool) {
        self.ready = ready;
    }
}

// Public API functions
pub fn create_input_stream(buffer: Vec<u8>) -> InputStream {
    let mut streams = STREAMS.lock();
    streams.create_input_stream(buffer)
}

pub fn create_output_stream() -> OutputStream {
    let mut streams = STREAMS.lock();
    streams.create_output_stream()
}

pub fn input_stream_read(id: InputStream, len: u64) -> Result<Vec<u8>, StreamError> {
    let mut streams = STREAMS.lock();
    if let Some(stream) = streams.get_input_stream(id) {
        stream.read(len)
    } else {
        Err(StreamError::Closed)
    }
}

pub fn input_stream_blocking_read(id: InputStream, len: u64) -> Result<Vec<u8>, StreamError> {
    let mut streams = STREAMS.lock();
    if let Some(stream) = streams.get_input_stream(id) {
        stream.blocking_read(len)
    } else {
        Err(StreamError::Closed)
    }
}

pub fn input_stream_skip(id: InputStream, len: u64) -> Result<u64, StreamError> {
    let mut streams = STREAMS.lock();
    if let Some(stream) = streams.get_input_stream(id) {
        stream.skip(len)
    } else {
        Err(StreamError::Closed)
    }
}

pub fn input_stream_blocking_skip(id: InputStream, len: u64) -> Result<u64, StreamError> {
    let mut streams = STREAMS.lock();
    if let Some(stream) = streams.get_input_stream(id) {
        stream.blocking_skip(len)
    } else {
        Err(StreamError::Closed)
    }
}

pub fn input_stream_subscribe(id: InputStream) -> Pollable {
    let streams = STREAMS.lock();
    if let Some(stream) = streams.input_streams.get(&id) {
        stream.subscribe()
    } else {
        let mut pollables = POLLABLES.lock();
        pollables.create_pollable(false)
    }
}

pub fn output_stream_check_write(id: OutputStream) -> Result<u64, StreamError> {
    let streams = STREAMS.lock();
    if let Some(stream) = streams.output_streams.get(&id) {
        stream.check_write()
    } else {
        Err(StreamError::Closed)
    }
}

pub fn output_stream_write(id: OutputStream, contents: Vec<u8>) -> Result<(), StreamError> {
    let mut streams = STREAMS.lock();
    if let Some(stream) = streams.get_output_stream(id) {
        stream.write(contents)
    } else {
        Err(StreamError::Closed)
    }
}

pub fn output_stream_blocking_write_and_flush(
    id: OutputStream,
    contents: Vec<u8>,
) -> Result<(), StreamError> {
    let mut streams = STREAMS.lock();
    if let Some(stream) = streams.get_output_stream(id) {
        stream.blocking_write_and_flush(contents)
    } else {
        Err(StreamError::Closed)
    }
}

pub fn output_stream_flush(id: OutputStream) -> Result<(), StreamError> {
    let mut streams = STREAMS.lock();
    if let Some(stream) = streams.get_output_stream(id) {
        stream.flush()
    } else {
        Err(StreamError::Closed)
    }
}

pub fn output_stream_blocking_flush(id: OutputStream) -> Result<(), StreamError> {
    let mut streams = STREAMS.lock();
    if let Some(stream) = streams.get_output_stream(id) {
        stream.blocking_flush()
    } else {
        Err(StreamError::Closed)
    }
}

pub fn output_stream_subscribe(id: OutputStream) -> Pollable {
    let streams = STREAMS.lock();
    if let Some(stream) = streams.output_streams.get(&id) {
        stream.subscribe()
    } else {
        let mut pollables = POLLABLES.lock();
        pollables.create_pollable(false)
    }
}

pub fn output_stream_write_zeroes(id: OutputStream, len: u64) -> Result<(), StreamError> {
    let mut streams = STREAMS.lock();
    if let Some(stream) = streams.get_output_stream(id) {
        stream.write_zeroes(len)
    } else {
        Err(StreamError::Closed)
    }
}

pub fn output_stream_blocking_write_zeroes_and_flush(
    id: OutputStream,
    len: u64,
) -> Result<(), StreamError> {
    let mut streams = STREAMS.lock();
    if let Some(stream) = streams.get_output_stream(id) {
        stream.blocking_write_zeroes_and_flush(len)
    } else {
        Err(StreamError::Closed)
    }
}

pub fn output_stream_splice(
    id: OutputStream,
    src: InputStream,
    len: u64,
) -> Result<u64, StreamError> {
    let mut streams = STREAMS.lock();
    let src_stream = streams.get_input_stream(src).ok_or(StreamError::Closed)?;
    let data = src_stream.read(len)?;
    let bytes_transferred = data.len() as u64;

    if let Some(dst_stream) = streams.get_output_stream(id) {
        dst_stream.write(data)?;
        Ok(bytes_transferred)
    } else {
        Err(StreamError::Closed)
    }
}

pub fn output_stream_blocking_splice(
    id: OutputStream,
    src: InputStream,
    len: u64,
) -> Result<u64, StreamError> {
    // In a real implementation, this would block until ready
    output_stream_splice(id, src, len)
}

pub fn pollable_ready(id: Pollable) -> bool {
    let pollables = POLLABLES.lock();
    if let Some(pollable) = pollables.pollables.get(&id) {
        pollable.ready()
    } else {
        false
    }
}

pub fn pollable_block(id: Pollable) {
    let mut pollables = POLLABLES.lock();
    if let Some(pollable) = pollables.get_pollable(id) {
        pollable.block();
    }
}

pub fn poll(pollables: &[Pollable]) -> Vec<u32> {
    let mut ready_indices = Vec::new();
    let pollable_registry = POLLABLES.lock();

    for (index, &pollable_id) in pollables.iter().enumerate() {
        if let Some(pollable) = pollable_registry.pollables.get(&pollable_id) {
            if pollable.ready() {
                ready_indices.push(index as u32);
            }
        }
    }

    // If nothing is ready, simulate waiting and return the first one as ready
    if ready_indices.is_empty() && !pollables.is_empty() {
        ready_indices.push(0);
    }

    ready_indices
}

pub fn drop_input_stream(id: InputStream) {
    let mut streams = STREAMS.lock();
    streams.remove_input_stream(id);
}

pub fn drop_output_stream(id: OutputStream) {
    let mut streams = STREAMS.lock();
    streams.remove_output_stream(id);
}

pub fn drop_pollable(id: Pollable) {
    let mut pollables = POLLABLES.lock();
    pollables.remove_pollable(id);
}

// Additional I/O functions for Preview 2 compatibility
pub fn read(_stream: InputStream, len: u64) -> WasiResult<(Vec<u8>, u8)> {
    let mut buffer = alloc::vec![0u8; len as usize];
    let bytes_read = crate::sys::fs::read_file(&alloc::format!("/dev/stdin"))
        .map(|data| data.len().min(len as usize))
        .unwrap_or(0);
    buffer.truncate(bytes_read);
    Ok((buffer, 0)) // 0 = continue status
}

pub fn blocking_read(stream: InputStream, len: u64) -> WasiResult<(Vec<u8>, u8)> {
    read(stream, len)
}

pub fn skip(_stream: InputStream, len: u64) -> WasiResult<(u64, u8)> {
    Ok((len, 0)) // Simulate skipping len bytes
}

pub fn blocking_skip(stream: InputStream, len: u64) -> WasiResult<(u64, u8)> {
    skip(stream, len)
}

pub fn subscribe_to_input_stream(_stream: InputStream) -> Pollable {
    let mut pollables = POLLABLES.lock();
    pollables.create_pollable(true) // Always ready for demo
}

pub fn check_write(_stream: OutputStream) -> WasiResult<u64> {
    Ok(1024) // Always allow 1KB writes
}

pub fn write(stream: OutputStream, contents: &[u8]) -> WasiResult<()> {
    // For demo, just pretend to write
    log::info!("Writing {} bytes to stream {}", contents.len(), stream);
    Ok(())
}

pub fn blocking_write_and_flush(stream: OutputStream, contents: &[u8]) -> WasiResult<()> {
    write(stream, contents)?;
    flush(stream)
}

pub fn flush(stream: OutputStream) -> WasiResult<()> {
    // For demo, just pretend to flush
    log::info!("Flushing stream {}", stream);
    Ok(())
}

pub fn blocking_flush(stream: OutputStream) -> WasiResult<()> {
    flush(stream)
}

pub fn subscribe_to_output_stream(_stream: OutputStream) -> Pollable {
    let mut pollables = POLLABLES.lock();
    pollables.create_pollable(true) // Always ready for demo
}

pub fn get_stdin() -> u32 {
    // Get stdin descriptor
    0
}

pub fn get_stdout() -> u32 {
    // Get stdout descriptor
    1
}

pub fn get_stderr() -> u32 {
    // Get stderr descriptor
    2
}
