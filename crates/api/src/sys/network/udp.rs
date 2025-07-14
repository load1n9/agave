/// UDP implementation for Agave OS
use crate::sys::error::{AgaveError, AgaveResult};
use alloc::{vec::Vec, collections::BTreeMap};
use core::net::SocketAddr;

/// UDP socket
#[derive(Debug)]
pub struct UdpSocket {
    pub local_addr: SocketAddr,
    pub recv_buffer: Vec<(Vec<u8>, SocketAddr)>, // (data, sender_addr)
}

impl UdpSocket {
    pub fn new(local_addr: SocketAddr) -> Self {
        Self {
            local_addr,
            recv_buffer: Vec::new(),
        }
    }

    pub fn bind(&mut self, addr: SocketAddr) -> AgaveResult<()> {
        self.local_addr = addr;
        Ok(())
    }

    pub fn send_to(&self, data: &[u8], remote_addr: SocketAddr) -> AgaveResult<usize> {
        // TODO: Actually send UDP packet
        log::trace!("UDP send {} bytes to {}", data.len(), remote_addr);
        Ok(data.len())
    }

    pub fn recv_from(&mut self, buffer: &mut [u8]) -> AgaveResult<(usize, SocketAddr)> {
        if let Some((data, sender)) = self.recv_buffer.pop() {
            let bytes_to_copy = buffer.len().min(data.len());
            buffer[..bytes_to_copy].copy_from_slice(&data[..bytes_to_copy]);
            Ok((bytes_to_copy, sender))
        } else {
            Err(AgaveError::Busy) // Would block
        }
    }

    pub fn receive_packet(&mut self, data: Vec<u8>, sender: SocketAddr) {
        self.recv_buffer.push((data, sender));
    }
}

/// UDP manager
pub struct UdpManager {
    sockets: BTreeMap<u64, UdpSocket>,
    next_socket_id: u64,
}

impl UdpManager {
    pub fn new() -> Self {
        Self {
            sockets: BTreeMap::new(),
            next_socket_id: 1,
        }
    }

    pub fn create_socket(&mut self, local_addr: SocketAddr) -> AgaveResult<u64> {
        let socket_id = self.next_socket_id;
        self.next_socket_id += 1;

        let socket = UdpSocket::new(local_addr);
        self.sockets.insert(socket_id, socket);
        
        Ok(socket_id)
    }

    pub fn get_socket(&mut self, socket_id: u64) -> Option<&mut UdpSocket> {
        self.sockets.get_mut(&socket_id)
    }

    pub fn close_socket(&mut self, socket_id: u64) -> AgaveResult<()> {
        self.sockets.remove(&socket_id);
        Ok(())
    }
}
