/// TCP implementation for Agave OS
use crate::sys::error::{AgaveError, AgaveResult};
use alloc::{vec::Vec, collections::BTreeMap};
use core::net::SocketAddr;

/// TCP socket state
#[derive(Debug, Clone, PartialEq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

/// TCP socket
#[derive(Debug)]
pub struct TcpSocket {
    pub local_addr: SocketAddr,
    pub remote_addr: Option<SocketAddr>,
    pub state: TcpState,
    pub send_buffer: Vec<u8>,
    pub recv_buffer: Vec<u8>,
    pub seq_num: u32,
    pub ack_num: u32,
}

impl TcpSocket {
    pub fn new(local_addr: SocketAddr) -> Self {
        Self {
            local_addr,
            remote_addr: None,
            state: TcpState::Closed,
            send_buffer: Vec::new(),
            recv_buffer: Vec::new(),
            seq_num: 0,
            ack_num: 0,
        }
    }

    pub fn bind(&mut self, addr: SocketAddr) -> AgaveResult<()> {
        self.local_addr = addr;
        Ok(())
    }

    pub fn listen(&mut self) -> AgaveResult<()> {
        self.state = TcpState::Listen;
        Ok(())
    }

    pub fn connect(&mut self, remote_addr: SocketAddr) -> AgaveResult<()> {
        self.remote_addr = Some(remote_addr);
        self.state = TcpState::SynSent;
        // TODO: Send SYN packet
        Ok(())
    }

    pub fn send(&mut self, data: &[u8]) -> AgaveResult<usize> {
        if self.state != TcpState::Established {
            return Err(AgaveError::InvalidInput);
        }
        
        self.send_buffer.extend_from_slice(data);
        // TODO: Actually send the data
        Ok(data.len())
    }

    pub fn receive(&mut self, buffer: &mut [u8]) -> AgaveResult<usize> {
        if self.state != TcpState::Established {
            return Err(AgaveError::InvalidInput);
        }

        let bytes_to_read = buffer.len().min(self.recv_buffer.len());
        buffer[..bytes_to_read].copy_from_slice(&self.recv_buffer[..bytes_to_read]);
        self.recv_buffer.drain(..bytes_to_read);
        
        Ok(bytes_to_read)
    }

    pub fn close(&mut self) -> AgaveResult<()> {
        match self.state {
            TcpState::Established => {
                self.state = TcpState::FinWait1;
                // TODO: Send FIN packet
            }
            _ => {}
        }
        Ok(())
    }
}

/// TCP manager
pub struct TcpManager {
    sockets: BTreeMap<u64, TcpSocket>,
    next_socket_id: u64,
}

impl TcpManager {
    pub fn new() -> Self {
        Self {
            sockets: BTreeMap::new(),
            next_socket_id: 1,
        }
    }

    pub fn create_socket(&mut self, local_addr: SocketAddr) -> AgaveResult<u64> {
        let socket_id = self.next_socket_id;
        self.next_socket_id += 1;

        let socket = TcpSocket::new(local_addr);
        self.sockets.insert(socket_id, socket);
        
        Ok(socket_id)
    }

    pub fn get_socket(&mut self, socket_id: u64) -> Option<&mut TcpSocket> {
        self.sockets.get_mut(&socket_id)
    }

    pub fn close_socket(&mut self, socket_id: u64) -> AgaveResult<()> {
        if let Some(mut socket) = self.sockets.remove(&socket_id) {
            socket.close()?;
        }
        Ok(())
    }
}
