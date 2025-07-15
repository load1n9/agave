/// Network stack implementation for Agave OS
/// Provides TCP/UDP networking, HTTP client/server, and network utilities
use crate::sys::error::{AgaveError, AgaveResult};
use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use spin::Mutex;

pub mod dns;
pub mod ethernet;
pub mod http;
pub mod protocols;
pub mod sockets;
pub mod tcp;
pub mod udp;

/// Network configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub ip_address: Ipv4Addr,
    pub subnet_mask: Ipv4Addr,
    pub gateway: Ipv4Addr,
    pub dns_servers: Vec<Ipv4Addr>,
    pub hostname: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            ip_address: Ipv4Addr::new(10, 0, 2, 15), // QEMU default
            subnet_mask: Ipv4Addr::new(255, 255, 255, 0),
            gateway: Ipv4Addr::new(10, 0, 2, 2),
            dns_servers: vec![Ipv4Addr::new(8, 8, 8, 8), Ipv4Addr::new(1, 1, 1, 1)],
            hostname: String::from("agave-os"),
        }
    }
}

/// Network interface state
#[derive(Debug, Clone)]
pub enum InterfaceState {
    Down,
    Up,
    Connecting,
    Connected,
    Error(String),
}

/// Network interface
#[derive(Debug)]
pub struct NetworkInterface {
    pub name: String,
    pub mac_address: [u8; 6],
    pub config: NetworkConfig,
    pub state: InterfaceState,
    pub stats: NetworkStats,
}

/// Network statistics
#[derive(Debug, Clone, Default)]
pub struct NetworkStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub errors: u64,
    pub dropped: u64,
}

/// Global network manager
pub static NETWORK_MANAGER: Mutex<NetworkManager> = Mutex::new(NetworkManager::new());

/// Network manager handles all network interfaces and routing
pub struct NetworkManager {
    interfaces: BTreeMap<String, NetworkInterface>,
    routing_table: Vec<Route>,
    arp_table: BTreeMap<Ipv4Addr, [u8; 6]>,
}

/// Network route
#[derive(Debug, Clone)]
pub struct Route {
    pub destination: Ipv4Addr,
    pub netmask: Ipv4Addr,
    pub gateway: Ipv4Addr,
    pub interface: String,
    pub metric: u32,
}

impl NetworkManager {
    const fn new() -> Self {
        Self {
            interfaces: BTreeMap::new(),
            routing_table: Vec::new(),
            arp_table: BTreeMap::new(),
        }
    }

    /// Add a network interface
    pub fn add_interface(&mut self, interface: NetworkInterface) -> AgaveResult<()> {
        log::info!("Adding network interface: {}", interface.name);
        self.interfaces.insert(interface.name.clone(), interface);
        Ok(())
    }

    /// Configure network interface
    pub fn configure_interface(&mut self, name: &str, config: NetworkConfig) -> AgaveResult<()> {
        if let Some(interface) = self.interfaces.get_mut(name) {
            interface.config = config;
            interface.state = InterfaceState::Down;
            log::info!(
                "Configured interface {}: IP {}",
                name,
                interface.config.ip_address
            );
            Ok(())
        } else {
            Err(AgaveError::NotFound)
        }
    }

    /// Bring interface up
    pub fn interface_up(&mut self, name: &str) -> AgaveResult<()> {
        // Get gateway first to avoid borrow conflict
        let gateway = if let Some(interface) = self.interfaces.get(name) {
            interface.config.gateway
        } else {
            return Err(AgaveError::NotFound);
        };

        // Now update the interface state
        if let Some(interface) = self.interfaces.get_mut(name) {
            interface.state = InterfaceState::Up;
            log::info!("Interface {} is now up", name);

            // Add default route
            self.add_route(Route {
                destination: Ipv4Addr::new(0, 0, 0, 0),
                netmask: Ipv4Addr::new(0, 0, 0, 0),
                gateway,
                interface: name.to_string(),
                metric: 100,
            })?;

            Ok(())
        } else {
            Err(AgaveError::NotFound)
        }
    }

    /// Add route to routing table
    pub fn add_route(&mut self, route: Route) -> AgaveResult<()> {
        log::info!(
            "Adding route: {} -> {} via {}",
            route.destination,
            route.interface,
            route.gateway
        );
        self.routing_table.push(route);
        self.routing_table.sort_by_key(|r| r.metric);
        Ok(())
    }

    /// Find route for destination
    pub fn find_route(&self, dest: Ipv4Addr) -> Option<&Route> {
        self.routing_table
            .iter()
            .find(|route| self.matches_route(dest, route))
    }

    fn matches_route(&self, dest: Ipv4Addr, route: &Route) -> bool {
        let dest_bits = u32::from(dest);
        let route_bits = u32::from(route.destination);
        let mask_bits = u32::from(route.netmask);

        (dest_bits & mask_bits) == (route_bits & mask_bits)
    }

    /// Get interface statistics
    pub fn get_stats(&self, interface: &str) -> Option<NetworkStats> {
        self.interfaces
            .get(interface)
            .map(|iface| iface.stats.clone())
    }

    /// List all interfaces
    pub fn list_interfaces(&self) -> Vec<(String, InterfaceState)> {
        self.interfaces
            .iter()
            .map(|(name, iface)| (name.clone(), iface.state.clone()))
            .collect()
    }
}

/// Public API functions
pub fn init_network() -> AgaveResult<()> {
    log::info!("Initializing network stack...");

    // Create default interface (will be configured by VirtIO driver)
    let default_interface = NetworkInterface {
        name: "eth0".to_string(),
        mac_address: [0x52, 0x54, 0x00, 0x12, 0x34, 0x56], // QEMU default
        config: NetworkConfig::default(),
        state: InterfaceState::Down,
        stats: NetworkStats::default(),
    };

    let mut manager = NETWORK_MANAGER.lock();
    manager.add_interface(default_interface)?;

    log::info!("Network stack initialized");
    Ok(())
}

/// Configure the default network interface
pub fn configure_default_interface() -> AgaveResult<()> {
    let mut manager = NETWORK_MANAGER.lock();
    let config = NetworkConfig::default();
    manager.configure_interface("eth0", config)?;
    manager.interface_up("eth0")?;
    Ok(())
}

/// Send raw packet
pub fn send_packet(interface: &str, packet: &[u8]) -> AgaveResult<()> {
    let mut manager = NETWORK_MANAGER.lock();
    if let Some(iface) = manager.interfaces.get_mut(interface) {
        // TODO: Implement actual packet transmission via VirtIO
        iface.stats.packets_sent += 1;
        iface.stats.bytes_sent += packet.len() as u64;
        log::trace!("Sent {} byte packet on {}", packet.len(), interface);
        Ok(())
    } else {
        Err(AgaveError::NotFound)
    }
}

/// Receive packet (called by driver)
pub fn receive_packet(interface: &str, packet: &[u8]) -> AgaveResult<()> {
    let mut manager = NETWORK_MANAGER.lock();
    if let Some(iface) = manager.interfaces.get_mut(interface) {
        iface.stats.packets_received += 1;
        iface.stats.bytes_received += packet.len() as u64;
        log::trace!("Received {} byte packet on {}", packet.len(), interface);

        // TODO: Process packet through protocol stack
        protocols::process_packet(packet)?;
        Ok(())
    } else {
        Err(AgaveError::NotFound)
    }
}

/// Get network statistics
pub fn get_network_stats() -> BTreeMap<String, NetworkStats> {
    let manager = NETWORK_MANAGER.lock();
    manager
        .interfaces
        .iter()
        .map(|(name, iface)| (name.clone(), iface.stats.clone()))
        .collect()
}

/// Check if network is available
pub fn is_network_available() -> bool {
    let manager = NETWORK_MANAGER.lock();
    manager
        .interfaces
        .values()
        .any(|iface| matches!(iface.state, InterfaceState::Up | InterfaceState::Connected))
}

/// Get local IP address
pub fn get_local_ip() -> Option<Ipv4Addr> {
    let manager = NETWORK_MANAGER.lock();
    manager
        .interfaces
        .values()
        .find(|iface| matches!(iface.state, InterfaceState::Up | InterfaceState::Connected))
        .map(|iface| iface.config.ip_address)
}

/// Resolve hostname to IP (simple DNS)
pub async fn resolve_hostname(hostname: &str) -> AgaveResult<Ipv4Addr> {
    match hostname {
        "localhost" => Ok(Ipv4Addr::new(127, 0, 0, 1)),
        "gateway" => {
            let manager = NETWORK_MANAGER.lock();
            Ok(manager
                .interfaces
                .values()
                .next()
                .map(|iface| iface.config.gateway)
                .unwrap_or(Ipv4Addr::new(10, 0, 2, 2)))
        }
        _ => {
            // For now, return a placeholder
            // TODO: Implement actual DNS resolution
            log::warn!("DNS resolution not yet implemented for: {}", hostname);
            Err(AgaveError::NotFound)
        }
    }
}
