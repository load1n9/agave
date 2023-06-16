use alloc::sync::Arc;
use alloc::vec::Vec;
use smoltcp::{
    iface::Routes,
    iface::SocketSet,
    time::Instant,
    wire::{EthernetAddress, IpAddress, IpCidr},
};
use spin::Mutex;

pub static NETWORK_INTERFACES: Mutex<Vec<NetworkInterfaceRef>> = Mutex::new(Vec::new());

pub trait NetworkInterface {
    fn ethernet_addr(&self) -> EthernetAddress;

    fn set_ethernet_addr(&mut self, addr: EthernetAddress);

    fn poll(&mut self, sockets: &mut SocketSet, timestamp: Instant) -> smoltcp::wire::Result<bool>;

    fn ip_addrs(&self) -> &[IpCidr];

    fn has_ip_addr(&self, addr: IpAddress) -> bool;

    fn routes(&self) -> &Routes;

    fn routes_mut(&mut self) -> &mut Routes;
}

pub type NetworkInterfaceRef = Arc<Mutex<dyn NetworkInterface + Send>>;

pub fn add_to_network_interfaces<T: NetworkInterface + 'static + Send>(iface: T) {
    NETWORK_INTERFACES.lock().push(Arc::new(Mutex::new(iface)));
}
