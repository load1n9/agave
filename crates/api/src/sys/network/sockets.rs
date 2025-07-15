/// Socket abstraction for network communication
use crate::sys::error::{AgaveError, AgaveResult};
use alloc::vec::Vec;
use core::net::SocketAddr;

/// Socket types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SocketType {
    Tcp,
    Udp,
    Raw,
}

/// Socket state
#[derive(Debug, Clone, PartialEq)]
pub enum SocketState {
    Closed,
    Bound,
    Listening,
    Connected,
    Error,
}

/// Generic socket interface
pub trait Socket {
    fn bind(&mut self, addr: SocketAddr) -> AgaveResult<()>;
    fn local_addr(&self) -> AgaveResult<SocketAddr>;
    fn socket_type(&self) -> SocketType;
    fn state(&self) -> SocketState;
}

/// Socket manager for all network sockets
pub struct SocketManager {
    tcp_sockets: alloc::collections::BTreeMap<u64, crate::sys::network::tcp::TcpSocket>,
    udp_sockets: alloc::collections::BTreeMap<u64, crate::sys::network::udp::UdpSocket>,
    next_socket_id: u64,
}

impl SocketManager {
    pub fn new() -> Self {
        Self {
            tcp_sockets: alloc::collections::BTreeMap::new(),
            udp_sockets: alloc::collections::BTreeMap::new(),
            next_socket_id: 1,
        }
    }

    /// Create a new socket
    pub fn create_socket(
        &mut self,
        socket_type: SocketType,
        local_addr: SocketAddr,
    ) -> AgaveResult<u64> {
        let socket_id = self.next_socket_id;
        self.next_socket_id += 1;

        match socket_type {
            SocketType::Tcp => {
                let socket = crate::sys::network::tcp::TcpSocket::new(local_addr);
                self.tcp_sockets.insert(socket_id, socket);
            }
            SocketType::Udp => {
                let socket = crate::sys::network::udp::UdpSocket::new(local_addr);
                self.udp_sockets.insert(socket_id, socket);
            }
            SocketType::Raw => {
                return Err(AgaveError::NotFound); // Not implemented yet
            }
        }

        Ok(socket_id)
    }

    /// Close a socket
    pub fn close_socket(&mut self, socket_id: u64) -> AgaveResult<()> {
        // Try TCP first
        if self.tcp_sockets.remove(&socket_id).is_some() {
            return Ok(());
        }

        // Try UDP
        if self.udp_sockets.remove(&socket_id).is_some() {
            return Ok(());
        }

        Err(AgaveError::NotFound)
    }

    /// Get TCP socket
    pub fn get_tcp_socket(
        &mut self,
        socket_id: u64,
    ) -> Option<&mut crate::sys::network::tcp::TcpSocket> {
        self.tcp_sockets.get_mut(&socket_id)
    }

    /// Get UDP socket
    pub fn get_udp_socket(
        &mut self,
        socket_id: u64,
    ) -> Option<&mut crate::sys::network::udp::UdpSocket> {
        self.udp_sockets.get_mut(&socket_id)
    }

    /// List all sockets
    pub fn list_sockets(&self) -> Vec<(u64, SocketType, SocketState)> {
        let mut sockets = Vec::new();

        for (id, socket) in &self.tcp_sockets {
            sockets.push((*id, SocketType::Tcp, socket.state.clone().into()));
        }

        for (id, _socket) in &self.udp_sockets {
            sockets.push((*id, SocketType::Udp, SocketState::Bound)); // UDP is always "bound"
        }

        sockets
    }
}

/// Convert TcpState to SocketState
impl From<crate::sys::network::tcp::TcpState> for SocketState {
    fn from(tcp_state: crate::sys::network::tcp::TcpState) -> Self {
        match tcp_state {
            crate::sys::network::tcp::TcpState::Closed => SocketState::Closed,
            crate::sys::network::tcp::TcpState::Listen => SocketState::Listening,
            crate::sys::network::tcp::TcpState::Established => SocketState::Connected,
            _ => SocketState::Bound,
        }
    }
}

/// Global socket manager
static mut SOCKET_MANAGER: Option<SocketManager> = None;

/// Initialize socket subsystem
pub fn init_sockets() -> AgaveResult<()> {
    unsafe {
        SOCKET_MANAGER = Some(SocketManager::new());
    }
    log::info!("Socket subsystem initialized");
    Ok(())
}

/// Create a socket
pub fn socket(socket_type: SocketType, local_addr: SocketAddr) -> AgaveResult<u64> {
    unsafe {
        #[allow(static_mut_refs)]
        if let Some(manager) = &mut SOCKET_MANAGER {
            manager.create_socket(socket_type, local_addr)
        } else {
            Err(AgaveError::NotFound)
        }
    }
}

/// Close a socket
pub fn close(socket_id: u64) -> AgaveResult<()> {
    unsafe {
        #[allow(static_mut_refs)]
        if let Some(manager) = &mut SOCKET_MANAGER {
            manager.close_socket(socket_id)
        } else {
            Err(AgaveError::NotFound)
        }
    }
}

/// Send data on TCP socket
pub fn tcp_send(socket_id: u64, data: &[u8]) -> AgaveResult<usize> {
    unsafe {
        #[allow(static_mut_refs)]
        if let Some(manager) = &mut SOCKET_MANAGER {
            if let Some(socket) = manager.get_tcp_socket(socket_id) {
                socket.send(data)
            } else {
                Err(AgaveError::NotFound)
            }
        } else {
            Err(AgaveError::NotFound)
        }
    }
}

/// Receive data from TCP socket
pub fn tcp_recv(socket_id: u64, buffer: &mut [u8]) -> AgaveResult<usize> {
    unsafe {
        #[allow(static_mut_refs)]
        if let Some(manager) = &mut SOCKET_MANAGER {
            if let Some(socket) = manager.get_tcp_socket(socket_id) {
                socket.receive(buffer)
            } else {
                Err(AgaveError::NotFound)
            }
        } else {
            Err(AgaveError::NotFound)
        }
    }
}

/// Send UDP packet
pub fn udp_send_to(socket_id: u64, data: &[u8], remote_addr: SocketAddr) -> AgaveResult<usize> {
    unsafe {
        #[allow(static_mut_refs)]
        if let Some(manager) = &mut SOCKET_MANAGER {
            if let Some(socket) = manager.get_udp_socket(socket_id) {
                socket.send_to(data, remote_addr)
            } else {
                Err(AgaveError::NotFound)
            }
        } else {
            Err(AgaveError::NotFound)
        }
    }
}

/// Receive UDP packet
pub fn udp_recv_from(socket_id: u64, buffer: &mut [u8]) -> AgaveResult<(usize, SocketAddr)> {
    unsafe {
        #[allow(static_mut_refs)]
        if let Some(manager) = &mut SOCKET_MANAGER {
            if let Some(socket) = manager.get_udp_socket(socket_id) {
                socket.recv_from(buffer)
            } else {
                Err(AgaveError::NotFound)
            }
        } else {
            Err(AgaveError::NotFound)
        }
    }
}

/// List all sockets
pub fn list_sockets() -> Vec<(u64, SocketType, SocketState)> {
    unsafe {
        #[allow(static_mut_refs)]
        if let Some(manager) = &SOCKET_MANAGER {
            manager.list_sockets()
        } else {
            Vec::new()
        }
    }
}
