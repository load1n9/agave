/// DNS client implementation for Agave OS
use crate::sys::error::{AgaveError, AgaveResult};
use alloc::{string::{String, ToString}, vec::Vec, collections::BTreeMap, vec};
use core::net::Ipv4Addr;

/// DNS record types
#[derive(Debug, Clone, PartialEq)]
pub enum DnsRecordType {
    A = 1,      // IPv4 address
    AAAA = 28,  // IPv6 address  
    CNAME = 5,  // Canonical name
    MX = 15,    // Mail exchange
    TXT = 16,   // Text record
}

/// DNS record
#[derive(Debug, Clone)]
pub struct DnsRecord {
    pub name: String,
    pub record_type: DnsRecordType,
    pub ttl: u32,
    pub data: Vec<u8>,
}

/// Simple DNS cache
pub struct DnsCache {
    records: BTreeMap<String, Vec<DnsRecord>>,
}

impl DnsCache {
    pub fn new() -> Self {
        Self {
            records: BTreeMap::new(),
        }
    }

    pub fn get(&self, hostname: &str, record_type: DnsRecordType) -> Option<&DnsRecord> {
        if let Some(records) = self.records.get(hostname) {
            records.iter().find(|r| r.record_type == record_type)
        } else {
            None
        }
    }

    pub fn insert(&mut self, record: DnsRecord) {
        self.records.entry(record.name.clone())
            .or_insert_with(Vec::new)
            .push(record);
    }

    pub fn clear_expired(&mut self, current_time: u64) {
        // TODO: Implement TTL-based expiration
        // For now, just log
        log::trace!("DNS cache cleanup at time {}", current_time);
    }
}

/// DNS resolver
pub struct DnsResolver {
    cache: DnsCache,
    dns_servers: Vec<Ipv4Addr>,
}

impl DnsResolver {
    pub fn new(dns_servers: Vec<Ipv4Addr>) -> Self {
        let mut resolver = Self {
            cache: DnsCache::new(),
            dns_servers,
        };

        // Add some static entries for demo
        resolver.add_static_entries();
        resolver
    }

    fn add_static_entries(&mut self) {
        // Add localhost entry
        let localhost_record = DnsRecord {
            name: "localhost".to_string(),
            record_type: DnsRecordType::A,
            ttl: 86400, // 24 hours
            data: vec![127, 0, 0, 1], // 127.0.0.1
        };
        self.cache.insert(localhost_record);

        // Add some demo entries
        let demo_entries = [
            ("example.com", [93, 184, 216, 34]),      // Example IP
            ("google.com", [8, 8, 8, 8]),             // Google DNS
            ("cloudflare.com", [1, 1, 1, 1]),         // Cloudflare DNS
        ];

        for (hostname, ip_bytes) in demo_entries.iter() {
            let record = DnsRecord {
                name: hostname.to_string(),
                record_type: DnsRecordType::A,
                ttl: 3600, // 1 hour
                data: ip_bytes.to_vec(),
            };
            self.cache.insert(record);
        }
    }

    pub async fn resolve(&mut self, hostname: &str) -> AgaveResult<Ipv4Addr> {
        // Check cache first
        if let Some(record) = self.cache.get(hostname, DnsRecordType::A) {
            if record.data.len() == 4 {
                return Ok(Ipv4Addr::new(
                    record.data[0],
                    record.data[1], 
                    record.data[2],
                    record.data[3],
                ));
            }
        }

        // Try to resolve via DNS servers
        for dns_server in &self.dns_servers {
            match self.query_dns_server(*dns_server, hostname).await {
                Ok(ip) => {
                    // Cache the result
                    let record = DnsRecord {
                        name: hostname.to_string(),
                        record_type: DnsRecordType::A,
                        ttl: 300, // 5 minutes
                        data: ip.octets().to_vec(),
                    };
                    self.cache.insert(record);
                    return Ok(ip);
                }
                Err(e) => {
                    log::warn!("DNS query to {} failed: {:?}", dns_server, e);
                }
            }
        }

        Err(AgaveError::NotFound)
    }

    async fn query_dns_server(&self, _dns_server: Ipv4Addr, hostname: &str) -> AgaveResult<Ipv4Addr> {
        // TODO: Implement actual DNS protocol (UDP port 53)
        log::trace!("Would query DNS server for: {}", hostname);
        
        // For now, return an error since we don't have network implementation yet
        Err(AgaveError::NotFound)
    }

    pub fn add_dns_server(&mut self, server: Ipv4Addr) {
        self.dns_servers.push(server);
    }

    pub fn clear_cache(&mut self) {
        self.cache = DnsCache::new();
        self.add_static_entries();
    }

    pub fn get_cache_stats(&self) -> (usize, usize) {
        let total_entries = self.cache.records.len();
        let total_records: usize = self.cache.records.values().map(|v| v.len()).sum();
        (total_entries, total_records)
    }
}

/// Global DNS resolver instance
static mut DNS_RESOLVER: Option<DnsResolver> = None;

/// Initialize DNS resolver
pub fn init_dns(dns_servers: Vec<Ipv4Addr>) -> AgaveResult<()> {
    log::info!("DNS resolver initialized with {} servers", dns_servers.len());
    unsafe {
        DNS_RESOLVER = Some(DnsResolver::new(dns_servers));
    }
    Ok(())
}

/// Resolve hostname to IP address
pub async fn resolve(hostname: &str) -> AgaveResult<Ipv4Addr> {
    unsafe {
        if let Some(resolver) = &mut DNS_RESOLVER {
            resolver.resolve(hostname).await
        } else {
            Err(AgaveError::NotFound)
        }
    }
}

/// Add a DNS server
pub fn add_dns_server(server: Ipv4Addr) -> AgaveResult<()> {
    unsafe {
        if let Some(resolver) = &mut DNS_RESOLVER {
            resolver.add_dns_server(server);
            Ok(())
        } else {
            Err(AgaveError::NotFound)
        }
    }
}

/// Clear DNS cache
pub fn clear_dns_cache() -> AgaveResult<()> {
    unsafe {
        if let Some(resolver) = &mut DNS_RESOLVER {
            resolver.clear_cache();
            Ok(())
        } else {
            Err(AgaveError::NotFound)
        }
    }
}
