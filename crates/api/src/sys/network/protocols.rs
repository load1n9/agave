/// Protocol processing for network stack
use crate::sys::error::{AgaveError, AgaveResult};
use alloc::vec::Vec;

/// Ethernet frame types
#[repr(u16)]
pub enum EtherType {
    IPv4 = 0x0800,
    IPv6 = 0x86DD,
    ARP = 0x0806,
}

/// IP protocol numbers
#[repr(u8)]
pub enum IpProtocol {
    ICMP = 1,
    TCP = 6,
    UDP = 17,
}

/// Ethernet header
#[repr(C, packed)]
pub struct EthernetHeader {
    pub dst_mac: [u8; 6],
    pub src_mac: [u8; 6],
    pub ether_type: u16,
}

/// IPv4 header
#[repr(C, packed)]
pub struct IPv4Header {
    pub version_ihl: u8,
    pub tos: u8,
    pub total_length: u16,
    pub identification: u16,
    pub flags_fragment: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub checksum: u16,
    pub src_ip: [u8; 4],
    pub dst_ip: [u8; 4],
}

/// Process incoming packet
pub fn process_packet(packet: &[u8]) -> AgaveResult<()> {
    if packet.len() < core::mem::size_of::<EthernetHeader>() {
        return Err(AgaveError::InvalidInput);
    }

    let eth_header = unsafe { &*(packet.as_ptr() as *const EthernetHeader) };
    let ether_type = u16::from_be(eth_header.ether_type);

    match ether_type {
        0x0800 => process_ipv4_packet(&packet[14..])?, // IPv4
        0x0806 => process_arp_packet(&packet[14..])?,  // ARP
        _ => {
            log::trace!("Unknown ethernet type: 0x{:04x}", ether_type);
        }
    }

    Ok(())
}

/// Process IPv4 packet
fn process_ipv4_packet(packet: &[u8]) -> AgaveResult<()> {
    if packet.len() < core::mem::size_of::<IPv4Header>() {
        return Err(AgaveError::InvalidInput);
    }

    let ip_header = unsafe { &*(packet.as_ptr() as *const IPv4Header) };
    let header_len = ((ip_header.version_ihl & 0x0F) * 4) as usize;

    if packet.len() < header_len {
        return Err(AgaveError::InvalidInput);
    }

    match ip_header.protocol {
        1 => process_icmp_packet(&packet[header_len..])?, // ICMP
        6 => process_tcp_packet(&packet[header_len..])?,  // TCP
        17 => process_udp_packet(&packet[header_len..])?, // UDP
        _ => {
            log::trace!("Unknown IP protocol: {}", ip_header.protocol);
        }
    }

    Ok(())
}

/// Process ICMP packet (ping responses, etc.)
fn process_icmp_packet(packet: &[u8]) -> AgaveResult<()> {
    if packet.len() < 8 {
        return Err(AgaveError::InvalidInput);
    }

    let icmp_type = packet[0];
    let icmp_code = packet[1];

    match icmp_type {
        0 => log::info!("ICMP Echo Reply received"),
        3 => log::info!("ICMP Destination Unreachable (code: {})", icmp_code),
        8 => log::info!("ICMP Echo Request received"),
        _ => log::trace!("ICMP type {} code {}", icmp_type, icmp_code),
    }

    Ok(())
}

/// Process TCP packet
fn process_tcp_packet(packet: &[u8]) -> AgaveResult<()> {
    if packet.len() < 20 {
        return Err(AgaveError::InvalidInput);
    }

    let src_port = u16::from_be_bytes([packet[0], packet[1]]);
    let dst_port = u16::from_be_bytes([packet[2], packet[3]]);
    let seq_num = u32::from_be_bytes([packet[4], packet[5], packet[6], packet[7]]);
    let flags = packet[13];

    log::trace!(
        "TCP: {}:{} -> {}:{} seq={} flags=0x{:02x}",
        "unknown",
        src_port,
        "local",
        dst_port,
        seq_num,
        flags
    );

    // TODO: Implement TCP state machine
    Ok(())
}

/// Process UDP packet
fn process_udp_packet(packet: &[u8]) -> AgaveResult<()> {
    if packet.len() < 8 {
        return Err(AgaveError::InvalidInput);
    }

    let src_port = u16::from_be_bytes([packet[0], packet[1]]);
    let dst_port = u16::from_be_bytes([packet[2], packet[3]]);
    let length = u16::from_be_bytes([packet[4], packet[5]]);

    log::trace!(
        "UDP: {}:{} -> {}:{} len={}",
        "unknown",
        src_port,
        "local",
        dst_port,
        length
    );

    // TODO: Deliver to UDP socket
    Ok(())
}

/// Process ARP packet
fn process_arp_packet(packet: &[u8]) -> AgaveResult<()> {
    if packet.len() < 28 {
        return Err(AgaveError::InvalidInput);
    }

    let operation = u16::from_be_bytes([packet[6], packet[7]]);

    match operation {
        1 => log::trace!("ARP Request received"),
        2 => log::trace!("ARP Reply received"),
        _ => log::trace!("Unknown ARP operation: {}", operation),
    }

    Ok(())
}

/// Calculate IPv4 checksum
pub fn calculate_ipv4_checksum(header: &[u8]) -> u16 {
    let mut sum: u32 = 0;

    for i in (0..header.len()).step_by(2) {
        if i + 1 < header.len() {
            let word = ((header[i] as u32) << 8) + (header[i + 1] as u32);
            sum += word;
        } else {
            sum += (header[i] as u32) << 8;
        }
    }

    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !sum as u16
}

/// Build Ethernet frame
pub fn build_ethernet_frame(
    dst_mac: [u8; 6],
    src_mac: [u8; 6],
    ether_type: u16,
    payload: &[u8],
) -> Vec<u8> {
    let mut frame = Vec::with_capacity(14 + payload.len());

    frame.extend_from_slice(&dst_mac);
    frame.extend_from_slice(&src_mac);
    frame.extend_from_slice(&ether_type.to_be_bytes());
    frame.extend_from_slice(payload);

    frame
}

/// Build IPv4 packet
pub fn build_ipv4_packet(
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    protocol: u8,
    payload: &[u8],
) -> Vec<u8> {
    let total_length = 20 + payload.len() as u16;
    let mut packet = Vec::with_capacity(total_length as usize);

    // IPv4 header
    packet.push(0x45); // Version 4, Header length 5 (20 bytes)
    packet.push(0); // TOS
    packet.extend_from_slice(&total_length.to_be_bytes());
    packet.extend_from_slice(&0u16.to_be_bytes()); // ID
    packet.extend_from_slice(&0u16.to_be_bytes()); // Flags + Fragment
    packet.push(64); // TTL
    packet.push(protocol);
    packet.extend_from_slice(&0u16.to_be_bytes()); // Checksum (will calculate)
    packet.extend_from_slice(&src_ip);
    packet.extend_from_slice(&dst_ip);

    // Calculate and insert checksum
    let checksum = calculate_ipv4_checksum(&packet[0..20]);
    packet[10] = (checksum >> 8) as u8;
    packet[11] = checksum as u8;

    // Add payload
    packet.extend_from_slice(payload);

    packet
}
