// WASI Sockets implementation for Agave OS
use super::error::*;
use super::types::*;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use spin::Mutex;

// Socket-specific types for Preview 2
pub type TcpSocket = u32;
pub type IpSocketAddress = Vec<u8>;
pub type IpAddressFamily = u8;

// Global socket registry
static SOCKETS: Mutex<SocketRegistry> = Mutex::new(SocketRegistry::new());

#[derive(Debug)]
pub struct SocketRegistry {
    sockets: BTreeMap<Fd, Socket>,
    next_fd: Fd,
}

impl SocketRegistry {
    pub const fn new() -> Self {
        Self {
            sockets: BTreeMap::new(),
            next_fd: 1000, // Start socket FDs at 1000
        }
    }

    pub fn allocate_fd(&mut self) -> Fd {
        let fd = self.next_fd;
        self.next_fd += 1;
        fd
    }

    pub fn create_socket(&mut self, socket_type: SocketType, address_family: AddressFamily) -> Fd {
        let fd = self.allocate_fd();
        self.sockets
            .insert(fd, Socket::new(socket_type, address_family));
        fd
    }

    pub fn get_socket(&mut self, fd: Fd) -> Option<&mut Socket> {
        self.sockets.get_mut(&fd)
    }

    pub fn remove_socket(&mut self, fd: Fd) -> Option<Socket> {
        self.sockets.remove(&fd)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SocketType {
    Stream,
    Datagram,
}

#[derive(Debug, Clone, Copy)]
pub enum AddressFamily {
    Inet4,
    Inet6,
    Unix,
}

// Core socket structures
#[derive(Debug, Clone)]
pub struct Network {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SocketState {
    Unbound,
    Bound,
    Listening,
    Connected,
    Closed,
}

#[derive(Debug, Clone)]
pub struct IpAddress {
    pub family: AddressFamily,
    pub addr: [u8; 16], // IPv6 size, IPv4 uses first 4 bytes
    pub port: u16,
}

impl IpAddress {
    pub fn new_ipv4(a: u8, b: u8, c: u8, d: u8, port: u16) -> Self {
        let mut addr = [0u8; 16];
        addr[0] = a;
        addr[1] = b;
        addr[2] = c;
        addr[3] = d;
        Self {
            family: AddressFamily::Inet4,
            addr,
            port,
        }
    }

    pub fn new_ipv6(addr: [u8; 16], port: u16) -> Self {
        Self {
            family: AddressFamily::Inet6,
            addr,
            port,
        }
    }

    pub fn loopback_ipv4(port: u16) -> Self {
        Self::new_ipv4(127, 0, 0, 1, port)
    }

    pub fn any_ipv4(port: u16) -> Self {
        Self::new_ipv4(0, 0, 0, 0, port)
    }
}

#[derive(Debug)]
pub struct Socket {
    socket_type: SocketType,
    address_family: AddressFamily,
    state: SocketState,
    local_address: Option<IpAddress>,
    remote_address: Option<IpAddress>,
    listen_queue: Vec<Fd>, // For listening sockets
    recv_buffer: Vec<u8>,
    send_buffer: Vec<u8>,
    _recv_timeout: Option<Timestamp>,
    _send_timeout: Option<Timestamp>,
    keep_alive: bool,
    no_delay: bool,
    reuse_address: bool,
}

impl Socket {
    pub fn new(socket_type: SocketType, address_family: AddressFamily) -> Self {
        Self {
            socket_type,
            address_family,
            state: SocketState::Unbound,
            local_address: None,
            remote_address: None,
            listen_queue: Vec::new(),
            recv_buffer: Vec::new(),
            send_buffer: Vec::new(),
            _recv_timeout: None,
            _send_timeout: None,
            keep_alive: false,
            no_delay: false,
            reuse_address: false,
        }
    }

    pub fn bind(&mut self, address: IpAddress) -> WasiResult<()> {
        if self.state != SocketState::Unbound {
            return Err(WasiError::inval());
        }

        // Check if address family matches
        if core::mem::discriminant(&address.family) != core::mem::discriminant(&self.address_family)
        {
            return Err(WasiError::inval());
        }

        self.local_address = Some(address);
        self.state = SocketState::Bound;
        Ok(())
    }

    pub fn listen(&mut self, backlog: u32) -> WasiResult<()> {
        if self.state != SocketState::Bound {
            return Err(WasiError::inval());
        }

        if !matches!(self.socket_type, SocketType::Stream) {
            return Err(WasiError::notsup());
        }

        self.state = SocketState::Listening;
        self.listen_queue.reserve(backlog as usize);
        Ok(())
    }

    pub fn connect(&mut self, address: IpAddress) -> WasiResult<()> {
        if self.state == SocketState::Connected {
            return Err(WasiError::already());
        }

        if self.state == SocketState::Closed {
            return Err(WasiError::badf());
        }

        // Check if address family matches
        if core::mem::discriminant(&address.family) != core::mem::discriminant(&self.address_family)
        {
            return Err(WasiError::inval());
        }

        self.remote_address = Some(address);
        self.state = SocketState::Connected;
        Ok(())
    }

    pub fn accept(&mut self) -> WasiResult<Option<(Fd, IpAddress)>> {
        if self.state != SocketState::Listening {
            return Err(WasiError::inval());
        }

        // In a real implementation, this would check for pending connections
        // For now, we'll simulate no pending connections
        Ok(None)
    }

    pub fn recv(&mut self, len: usize) -> WasiResult<Vec<u8>> {
        if self.state != SocketState::Connected {
            return Err(WasiError::notconn());
        }

        let data_len = len.min(self.recv_buffer.len());
        let data = self.recv_buffer.drain(..data_len).collect();
        Ok(data)
    }

    pub fn send(&mut self, data: &[u8]) -> WasiResult<usize> {
        if self.state != SocketState::Connected {
            return Err(WasiError::notconn());
        }

        // In a real implementation, this would send data over the network
        // For now, we'll just add to send buffer
        self.send_buffer.extend_from_slice(data);
        Ok(data.len())
    }

    pub fn shutdown(&mut self, how: u8) -> WasiResult<()> {
        match how {
            0 => { // SHUT_RD
                 // Close read side
            }
            1 => { // SHUT_WR
                 // Close write side
            }
            2 => {
                // SHUT_RDWR
                // Close both sides
                self.state = SocketState::Closed;
            }
            _ => return Err(WasiError::inval()),
        }
        Ok(())
    }

    pub fn close(&mut self) {
        self.state = SocketState::Closed;
        self.recv_buffer.clear();
        self.send_buffer.clear();
        self.listen_queue.clear();
    }
}

// Public API functions

// Create a new socket
pub fn socket(address_family: u8, socket_type: u8) -> WasiResult<Fd> {
    let af = match address_family {
        2 => AddressFamily::Inet4,  // AF_INET
        10 => AddressFamily::Inet6, // AF_INET6
        1 => AddressFamily::Unix,   // AF_UNIX
        _ => return Err(WasiError::inval()),
    };

    let st = match socket_type {
        1 => SocketType::Stream,   // SOCK_STREAM
        2 => SocketType::Datagram, // SOCK_DGRAM
        _ => return Err(WasiError::inval()),
    };

    let mut sockets = SOCKETS.lock();
    Ok(sockets.create_socket(st, af))
}

// Bind socket to an address
pub fn bind(fd: Fd, address: &[u8]) -> WasiResult<()> {
    let ip_addr = parse_socket_address(address)?;

    let mut sockets = SOCKETS.lock();
    if let Some(socket) = sockets.get_socket(fd) {
        socket.bind(ip_addr)
    } else {
        Err(WasiError::badf())
    }
}

// Listen for connections
pub fn listen(fd: Fd, backlog: u32) -> WasiResult<()> {
    let mut sockets = SOCKETS.lock();
    if let Some(socket) = sockets.get_socket(fd) {
        socket.listen(backlog)
    } else {
        Err(WasiError::badf())
    }
}

// Accept a connection
pub fn accept(fd: Fd) -> WasiResult<Option<(Fd, Vec<u8>)>> {
    let mut sockets = SOCKETS.lock();
    if let Some(socket) = sockets.get_socket(fd) {
        if let Some((client_fd, remote_addr)) = socket.accept()? {
            let addr_bytes = format_socket_address(&remote_addr)?;
            Ok(Some((client_fd, addr_bytes)))
        } else {
            Ok(None)
        }
    } else {
        Err(WasiError::badf())
    }
}

// Connect to a remote address
pub fn connect(fd: Fd, address: &[u8]) -> WasiResult<()> {
    let ip_addr = parse_socket_address(address)?;

    let mut sockets = SOCKETS.lock();
    if let Some(socket) = sockets.get_socket(fd) {
        socket.connect(ip_addr)
    } else {
        Err(WasiError::badf())
    }
}

// Receive data
pub fn recv(fd: Fd, buf_len: usize, _flags: RiFlags) -> WasiResult<Vec<u8>> {
    let mut sockets = SOCKETS.lock();
    if let Some(socket) = sockets.get_socket(fd) {
        socket.recv(buf_len)
    } else {
        Err(WasiError::badf())
    }
}

// Send data
pub fn send(fd: Fd, data: &[u8], _flags: SiFlags) -> WasiResult<usize> {
    let mut sockets = SOCKETS.lock();
    if let Some(socket) = sockets.get_socket(fd) {
        socket.send(data)
    } else {
        Err(WasiError::badf())
    }
}

// Shutdown socket
pub fn shutdown(fd: Fd, how: SdFlags) -> WasiResult<()> {
    let mut sockets = SOCKETS.lock();
    if let Some(socket) = sockets.get_socket(fd) {
        socket.shutdown(how)
    } else {
        Err(WasiError::badf())
    }
}

// Close socket
pub fn close_socket(fd: Fd) -> WasiResult<()> {
    let mut sockets = SOCKETS.lock();
    if let Some(mut socket) = sockets.remove_socket(fd) {
        socket.close();
        Ok(())
    } else {
        Err(WasiError::badf())
    }
}

// Get socket option
pub fn get_socket_option(fd: Fd, level: u32, name: u32) -> WasiResult<Vec<u8>> {
    let sockets = SOCKETS.lock();
    if let Some(socket) = sockets.sockets.get(&fd) {
        match (level, name) {
            (1, 1) => Ok(vec![if socket.reuse_address { 1 } else { 0 }]), // SO_REUSEADDR
            (1, 9) => Ok(vec![if socket.keep_alive { 1 } else { 0 }]),    // SO_KEEPALIVE
            (6, 1) => Ok(vec![if socket.no_delay { 1 } else { 0 }]),      // TCP_NODELAY
            _ => Err(WasiError::notsup()),
        }
    } else {
        Err(WasiError::badf())
    }
}

// Set socket option
pub fn set_socket_option(fd: Fd, level: u32, name: u32, value: &[u8]) -> WasiResult<()> {
    let mut sockets = SOCKETS.lock();
    if let Some(socket) = sockets.get_socket(fd) {
        if value.is_empty() {
            return Err(WasiError::inval());
        }

        let bool_val = value[0] != 0;

        match (level, name) {
            (1, 1) => socket.reuse_address = bool_val, // SO_REUSEADDR
            (1, 9) => socket.keep_alive = bool_val,    // SO_KEEPALIVE
            (6, 1) => socket.no_delay = bool_val,      // TCP_NODELAY
            _ => return Err(WasiError::notsup()),
        }
        Ok(())
    } else {
        Err(WasiError::badf())
    }
}

// Get local address
pub fn get_local_address(fd: Fd) -> WasiResult<Vec<u8>> {
    let sockets = SOCKETS.lock();
    if let Some(socket) = sockets.sockets.get(&fd) {
        if let Some(addr) = &socket.local_address {
            format_socket_address(addr)
        } else {
            Err(WasiError::notconn())
        }
    } else {
        Err(WasiError::badf())
    }
}

// Get remote address
pub fn get_remote_address(fd: Fd) -> WasiResult<Vec<u8>> {
    let sockets = SOCKETS.lock();
    if let Some(socket) = sockets.sockets.get(&fd) {
        if let Some(addr) = &socket.remote_address {
            format_socket_address(addr)
        } else {
            Err(WasiError::notconn())
        }
    } else {
        Err(WasiError::badf())
    }
}

// Helper functions
fn parse_socket_address(data: &[u8]) -> WasiResult<IpAddress> {
    if data.len() < 6 {
        return Err(WasiError::inval());
    }

    let family = u16::from_le_bytes([data[0], data[1]]);
    let port = u16::from_be_bytes([data[2], data[3]]);

    match family {
        2 => {
            // AF_INET
            if data.len() < 8 {
                return Err(WasiError::inval());
            }
            Ok(IpAddress::new_ipv4(
                data[4], data[5], data[6], data[7], port,
            ))
        }
        10 => {
            // AF_INET6
            if data.len() < 20 {
                return Err(WasiError::inval());
            }
            let mut addr = [0u8; 16];
            addr.copy_from_slice(&data[4..20]);
            Ok(IpAddress::new_ipv6(addr, port))
        }
        _ => Err(WasiError::inval()),
    }
}

fn format_socket_address(addr: &IpAddress) -> WasiResult<Vec<u8>> {
    let mut result = Vec::new();

    match addr.family {
        AddressFamily::Inet4 => {
            result.extend_from_slice(&2u16.to_le_bytes()); // AF_INET
            result.extend_from_slice(&addr.port.to_be_bytes());
            result.extend_from_slice(&addr.addr[0..4]);
        }
        AddressFamily::Inet6 => {
            result.extend_from_slice(&10u16.to_le_bytes()); // AF_INET6
            result.extend_from_slice(&addr.port.to_be_bytes());
            result.extend_from_slice(&addr.addr);
        }
        AddressFamily::Unix => {
            result.extend_from_slice(&1u16.to_le_bytes()); // AF_UNIX
                                                           // Unix sockets would need path handling
        }
    }

    Ok(result)
}

// Preview 2 API extensions
pub fn create_tcp_socket(address_family: u8) -> WasiResult<Fd> {
    socket(address_family, 1) // SOCK_STREAM
}

pub fn create_udp_socket(address_family: u8) -> WasiResult<Fd> {
    socket(address_family, 2) // SOCK_DGRAM
}

pub fn subscribe_to_socket(fd: Fd, _interest: u8) -> WasiResult<super::io::Pollable> {
    let sockets = SOCKETS.lock();
    if sockets.sockets.contains_key(&fd) {
        // Create a pollable for socket readiness
        let mut pollables = super::io::POLLABLES.lock();
        Ok(pollables.create_pollable(true)) // Always ready for now
    } else {
        Err(WasiError::badf())
    }
}

// Network interface information
pub fn get_network_interfaces() -> WasiResult<Vec<NetworkInterface>> {
    // Return basic loopback interface
    Ok(vec![NetworkInterface {
        name: "lo".to_string(),
        addresses: vec![IpAddress::loopback_ipv4(0)],
        is_up: true,
        is_loopback: true,
    }])
}

#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub addresses: Vec<IpAddress>,
    pub is_up: bool,
    pub is_loopback: bool,
}

// DNS resolution (basic implementation)
pub fn resolve_hostname(hostname: &str, address_family: u8) -> WasiResult<Vec<IpAddress>> {
    match hostname {
        "localhost" => {
            match address_family {
                2 => Ok(vec![IpAddress::loopback_ipv4(0)]), // IPv4
                10 => Ok(vec![IpAddress::new_ipv6(
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
                    0,
                )]), // IPv6
                _ => Err(WasiError::inval()),
            }
        }
        _ => {
            // In a real implementation, this would perform DNS lookup
            Err(WasiError::hostunreach())
        }
    }
}

// Additional functions for demo compatibility
pub fn instance_network() -> Network {
    Network {
        id: 0,
        name: "default".to_string(),
    }
}

// Additional socket functions for Preview 2 compatibility
pub fn subscribe(socket: TcpSocket) -> WasiResult<super::io::Pollable> {
    // Subscribe to socket events
    log::debug!("sockets::subscribe({})", socket);
    Ok(1)
}

pub fn shutdown_tcp(socket: TcpSocket, shutdown_type: u8) -> WasiResult<()> {
    // Shutdown TCP socket
    log::debug!("sockets::shutdown_tcp({}, {})", socket, shutdown_type);
    Ok(())
}

pub fn drop_tcp_socket(socket: TcpSocket) -> WasiResult<()> {
    // Drop TCP socket
    log::debug!("sockets::drop_tcp_socket({})", socket);
    Ok(())
}

pub fn drop_network(network: Network) -> WasiResult<()> {
    log::info!("Dropping network {}", network.name);
    Ok(())
}

pub fn start_bind(
    socket: TcpSocket,
    network: Network,
    local_address: IpSocketAddress,
) -> WasiResult<()> {
    log::info!(
        "Start bind socket {} on network {} to {:?}",
        socket,
        network.name,
        local_address
    );
    Ok(())
}

pub fn finish_bind(socket: TcpSocket) -> WasiResult<()> {
    log::info!("Finish bind socket {}", socket);
    Ok(())
}

pub fn start_connect(
    socket: TcpSocket,
    network: Network,
    remote_address: IpSocketAddress,
) -> WasiResult<()> {
    log::info!(
        "Start connect socket {} on network {} to {:?}",
        socket,
        network.name,
        remote_address
    );
    Ok(())
}

pub fn finish_connect(socket: TcpSocket) -> WasiResult<(InputStream, OutputStream)> {
    log::info!("Finish connect socket {}", socket);
    Ok((socket, socket)) // Use socket ID as stream IDs
}

pub fn start_listen(socket: TcpSocket) -> WasiResult<()> {
    log::info!("Start listen socket {}", socket);
    Ok(())
}

pub fn finish_listen(socket: TcpSocket) -> WasiResult<()> {
    log::info!("Finish listen socket {}", socket);
    Ok(())
}

pub fn accept_tcp(socket: TcpSocket) -> WasiResult<Option<(TcpSocket, InputStream, OutputStream)>> {
    log::info!("Accept TCP socket {}", socket);
    // For demo, return None (no pending connections)
    Ok(None)
}

pub fn local_address(socket: TcpSocket) -> WasiResult<IpSocketAddress> {
    get_local_address(socket)
}

pub fn remote_address(socket: TcpSocket) -> WasiResult<IpSocketAddress> {
    get_remote_address(socket)
}

pub fn is_listening(socket: TcpSocket) -> bool {
    log::info!("Check if socket {} is listening", socket);
    false // For demo, assume not listening
}

pub fn address_family(socket: TcpSocket) -> IpAddressFamily {
    log::info!("Get address family for socket {}", socket);
    4 // IPv4
}

pub fn set_listen_backlog_size(socket: TcpSocket, value: u64) -> WasiResult<()> {
    log::info!("Set listen backlog size {} for socket {}", value, socket);
    Ok(())
}

pub fn keep_alive_enabled(socket: TcpSocket) -> WasiResult<bool> {
    log::info!("Check keep alive enabled for socket {}", socket);
    Ok(false)
}

pub fn set_keep_alive_enabled(socket: TcpSocket, value: bool) -> WasiResult<()> {
    log::info!("Set keep alive enabled {} for socket {}", value, socket);
    Ok(())
}

pub fn keep_alive_idle_time(socket: TcpSocket) -> WasiResult<core::time::Duration> {
    log::info!("Get keep alive idle time for socket {}", socket);
    Ok(core::time::Duration::from_secs(60))
}

pub fn set_keep_alive_idle_time(socket: TcpSocket, value: core::time::Duration) -> WasiResult<()> {
    log::info!("Set keep alive idle time {:?} for socket {}", value, socket);
    Ok(())
}

pub fn keep_alive_interval(socket: TcpSocket) -> WasiResult<core::time::Duration> {
    log::info!("Get keep alive interval for socket {}", socket);
    Ok(core::time::Duration::from_secs(30))
}

pub fn set_keep_alive_interval(socket: TcpSocket, value: core::time::Duration) -> WasiResult<()> {
    log::info!("Set keep alive interval {:?} for socket {}", value, socket);
    Ok(())
}

pub fn keep_alive_count(socket: TcpSocket) -> WasiResult<u32> {
    log::info!("Get keep alive count for socket {}", socket);
    Ok(9)
}

pub fn set_keep_alive_count(socket: TcpSocket, value: u32) -> WasiResult<()> {
    log::info!("Set keep alive count {} for socket {}", value, socket);
    Ok(())
}

pub fn hop_limit(socket: TcpSocket) -> WasiResult<u8> {
    log::info!("Get hop limit for socket {}", socket);
    Ok(64)
}

pub fn set_hop_limit(socket: TcpSocket, value: u8) -> WasiResult<()> {
    log::info!("Set hop limit {} for socket {}", value, socket);
    Ok(())
}

pub fn receive_buffer_size(socket: TcpSocket) -> WasiResult<u64> {
    log::info!("Get receive buffer size for socket {}", socket);
    Ok(65536)
}

pub fn set_receive_buffer_size(socket: TcpSocket, value: u64) -> WasiResult<()> {
    log::info!("Set receive buffer size {} for socket {}", value, socket);
    Ok(())
}

pub fn send_buffer_size(socket: TcpSocket) -> WasiResult<u64> {
    log::info!("Get send buffer size for socket {}", socket);
    Ok(65536)
}

pub fn set_send_buffer_size(socket: TcpSocket, value: u64) -> WasiResult<()> {
    log::info!("Set send buffer size {} for socket {}", value, socket);
    Ok(())
}
