use super::wasi::error::WasiError;
use alloc::vec::Vec;

pub trait FileIO {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()>;
    fn write(&mut self, buf: &[u8]) -> Result<usize, ()>;
}

// Simple file reading function for WASI compatibility
pub fn read_file(path: &str) -> Result<Vec<u8>, WasiError> {
    // Mock file read for now
    log::debug!("fs::read_file({})", path);
    if path == "/dev/stdin" {
        // Return empty for stdin
        Ok(alloc::vec![])
    } else {
        // Return demo content
        Ok(alloc::vec![b'H', b'e', b'l', b'l', b'o'])
    }
}
