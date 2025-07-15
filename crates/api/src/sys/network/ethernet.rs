/// Ethernet frame handling for Agave OS
use crate::sys::error::{AgaveError, AgaveResult};
use alloc::vec::Vec;

/// Ethernet frame header
#[repr(C, packed)]
pub struct EthernetFrame {
    pub dst_mac: [u8; 6],
    pub src_mac: [u8; 6],
    pub ethertype: u16,
}

impl EthernetFrame {
    pub const MIN_FRAME_SIZE: usize = 64;
    pub const MAX_FRAME_SIZE: usize = 1518;
    pub const HEADER_SIZE: usize = 14;

    /// Create new ethernet frame
    pub fn new(dst_mac: [u8; 6], src_mac: [u8; 6], ethertype: u16) -> Self {
        Self {
            dst_mac,
            src_mac,
            ethertype: ethertype.to_be(),
        }
    }

    /// Parse ethernet frame from bytes
    pub fn parse(data: &[u8]) -> AgaveResult<(Self, &[u8])> {
        if data.len() < Self::HEADER_SIZE {
            return Err(AgaveError::InvalidInput);
        }

        let frame = Self {
            dst_mac: [data[0], data[1], data[2], data[3], data[4], data[5]],
            src_mac: [data[6], data[7], data[8], data[9], data[10], data[11]],
            ethertype: u16::from_be_bytes([data[12], data[13]]),
        };

        let payload = &data[Self::HEADER_SIZE..];
        Ok((frame, payload))
    }

    /// Convert frame to bytes
    pub fn to_bytes(&self, payload: &[u8]) -> Vec<u8> {
        let mut frame = Vec::with_capacity(Self::HEADER_SIZE + payload.len());

        frame.extend_from_slice(&self.dst_mac);
        frame.extend_from_slice(&self.src_mac);
        frame.extend_from_slice(&self.ethertype.to_be_bytes());
        frame.extend_from_slice(payload);

        // Pad frame to minimum size if necessary
        while frame.len() < Self::MIN_FRAME_SIZE {
            frame.push(0);
        }

        frame
    }

    /// Check if destination is broadcast
    pub fn is_broadcast(&self) -> bool {
        self.dst_mac == [0xFF; 6]
    }

    /// Check if destination is multicast
    pub fn is_multicast(&self) -> bool {
        self.dst_mac[0] & 0x01 != 0
    }

    /// Get ethertype in host byte order
    pub fn ethertype(&self) -> u16 {
        u16::from_be(self.ethertype)
    }
}

/// Common ethernet types
pub mod ethertypes {
    pub const IPV4: u16 = 0x0800;
    pub const IPV6: u16 = 0x86DD;
    pub const ARP: u16 = 0x0806;
    pub const RARP: u16 = 0x8035;
    pub const VLAN: u16 = 0x8100;
}

/// MAC address utilities
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    pub const BROADCAST: MacAddress = MacAddress([0xFF; 6]);
    pub const ZERO: MacAddress = MacAddress([0x00; 6]);

    pub fn new(bytes: [u8; 6]) -> Self {
        MacAddress(bytes)
    }

    pub fn is_broadcast(&self) -> bool {
        self.0 == [0xFF; 6]
    }

    pub fn is_multicast(&self) -> bool {
        self.0[0] & 0x01 != 0
    }

    pub fn is_unicast(&self) -> bool {
        !self.is_multicast()
    }

    pub fn is_locally_administered(&self) -> bool {
        self.0[0] & 0x02 != 0
    }

    pub fn bytes(&self) -> [u8; 6] {
        self.0
    }
}

impl core::fmt::Display for MacAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl core::fmt::Debug for MacAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "MacAddress({})", self)
    }
}

/// Ethernet frame builder
pub struct EthernetFrameBuilder {
    dst_mac: [u8; 6],
    src_mac: [u8; 6],
    ethertype: u16,
}

impl EthernetFrameBuilder {
    pub fn new() -> Self {
        Self {
            dst_mac: [0; 6],
            src_mac: [0; 6],
            ethertype: 0,
        }
    }

    pub fn dst_mac(mut self, mac: [u8; 6]) -> Self {
        self.dst_mac = mac;
        self
    }

    pub fn src_mac(mut self, mac: [u8; 6]) -> Self {
        self.src_mac = mac;
        self
    }

    pub fn ethertype(mut self, ethertype: u16) -> Self {
        self.ethertype = ethertype;
        self
    }

    pub fn build(self, payload: &[u8]) -> Vec<u8> {
        let frame = EthernetFrame::new(self.dst_mac, self.src_mac, self.ethertype);
        frame.to_bytes(payload)
    }
}
