//! STUN Client for NAT Traversal
//!
//! Implements basic STUN (RFC 5389) for discovering public IP and port.
//! This enables ELARA nodes behind NAT to communicate.

use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::time::timeout;

use elara_core::{ElaraError, ElaraResult};

/// STUN message types
const STUN_BINDING_REQUEST: u16 = 0x0001;
const STUN_BINDING_RESPONSE: u16 = 0x0101;

/// STUN attribute types
const STUN_ATTR_MAPPED_ADDRESS: u16 = 0x0001;
const STUN_ATTR_XOR_MAPPED_ADDRESS: u16 = 0x0020;

/// STUN magic cookie (RFC 5389)
const STUN_MAGIC_COOKIE: u32 = 0x2112A442;

/// STUN header size
const STUN_HEADER_SIZE: usize = 20;

/// Public STUN servers
pub const STUN_SERVERS: &[&str] = &[
    "stun.l.google.com:19302",
    "stun1.l.google.com:19302",
    "stun2.l.google.com:19302",
    "stun.cloudflare.com:3478",
];

/// Result of a STUN binding request
#[derive(Debug, Clone)]
pub struct StunResult {
    /// Server-reflexive address (public IP:port as seen by STUN server)
    pub mapped_address: SocketAddr,
    
    /// Local address used
    pub local_address: SocketAddr,
    
    /// STUN server used
    pub server: SocketAddr,
    
    /// Round-trip time
    pub rtt: Duration,
}

/// STUN client for NAT traversal
pub struct StunClient {
    /// Timeout for STUN requests
    timeout: Duration,
    
    /// Number of retries
    retries: u32,
}

impl StunClient {
    /// Create a new STUN client with default settings
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(3),
            retries: 3,
        }
    }

    /// Create with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            timeout,
            retries: 3,
        }
    }

    /// Discover public address using a specific STUN server
    pub async fn discover(&self, stun_server: &str) -> ElaraResult<StunResult> {
        let server_addr: SocketAddr = stun_server
            .parse()
            .map_err(|e| ElaraError::TransportError(format!("Invalid STUN server: {}", e)))?;

        self.discover_with_addr(server_addr).await
    }

    /// Discover public address using a SocketAddr
    pub async fn discover_with_addr(&self, server_addr: SocketAddr) -> ElaraResult<StunResult> {
        // Bind to any available port
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| ElaraError::TransportError(e.to_string()))?;

        let local_addr = socket
            .local_addr()
            .map_err(|e| ElaraError::TransportError(e.to_string()))?;

        // Build STUN binding request
        let transaction_id = generate_transaction_id();
        let request = build_binding_request(&transaction_id);

        // Send request with retries
        for attempt in 0..self.retries {
            let start = std::time::Instant::now();

            // Send request
            socket
                .send_to(&request, server_addr)
                .await
                .map_err(|e| ElaraError::TransportError(e.to_string()))?;

            // Wait for response
            let mut buf = [0u8; 512];
            match timeout(self.timeout, socket.recv_from(&mut buf)).await {
                Ok(Ok((len, from))) => {
                    if from == server_addr {
                        if let Some(mapped) = parse_binding_response(&buf[..len], &transaction_id) {
                            return Ok(StunResult {
                                mapped_address: mapped,
                                local_address: local_addr,
                                server: server_addr,
                                rtt: start.elapsed(),
                            });
                        }
                    }
                }
                Ok(Err(e)) => {
                    if attempt == self.retries - 1 {
                        return Err(ElaraError::TransportError(format!(
                            "STUN receive error: {}",
                            e
                        )));
                    }
                }
                Err(_) => {
                    if attempt == self.retries - 1 {
                        return Err(ElaraError::TransportError("STUN timeout".to_string()));
                    }
                }
            }
        }

        Err(ElaraError::TransportError(
            "STUN discovery failed after retries".to_string(),
        ))
    }

    /// Try multiple STUN servers and return the first successful result
    pub async fn discover_any(&self) -> ElaraResult<StunResult> {
        for server in STUN_SERVERS {
            match self.discover(server).await {
                Ok(result) => return Ok(result),
                Err(_) => continue,
            }
        }

        Err(ElaraError::TransportError(
            "All STUN servers failed".to_string(),
        ))
    }

    /// Check NAT type by comparing results from multiple servers
    pub async fn detect_nat_type(&self) -> ElaraResult<NatType> {
        let mut results = Vec::new();

        for server in STUN_SERVERS.iter().take(2) {
            if let Ok(result) = self.discover(server).await {
                results.push(result);
            }
        }

        if results.is_empty() {
            return Err(ElaraError::TransportError(
                "Could not contact any STUN servers".to_string(),
            ));
        }

        if results.len() == 1 {
            let result = &results[0];
            if result.mapped_address == result.local_address {
                return Ok(NatType::NoNat);
            }
            return Ok(NatType::Unknown);
        }

        // Compare mapped addresses from different servers
        let addr1 = results[0].mapped_address;
        let addr2 = results[1].mapped_address;

        if addr1.ip() == addr2.ip() && addr1.port() == addr2.port() {
            // Same external address from different servers
            Ok(NatType::FullCone)
        } else if addr1.ip() == addr2.ip() {
            // Same IP but different ports
            Ok(NatType::SymmetricPortOnly)
        } else {
            // Different IPs - symmetric NAT
            Ok(NatType::Symmetric)
        }
    }
}

impl Default for StunClient {
    fn default() -> Self {
        Self::new()
    }
}

/// NAT type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatType {
    /// No NAT - public IP
    NoNat,
    /// Full cone NAT - easiest to traverse
    FullCone,
    /// Restricted cone NAT
    RestrictedCone,
    /// Port restricted cone NAT
    PortRestrictedCone,
    /// Symmetric NAT - hardest to traverse
    Symmetric,
    /// Symmetric NAT with port-only variation
    SymmetricPortOnly,
    /// Could not determine
    Unknown,
}

impl NatType {
    /// Check if this NAT type is traversable with STUN alone
    pub fn stun_traversable(&self) -> bool {
        matches!(
            self,
            NatType::NoNat | NatType::FullCone | NatType::RestrictedCone | NatType::PortRestrictedCone
        )
    }

    /// Check if TURN is required
    pub fn requires_turn(&self) -> bool {
        matches!(self, NatType::Symmetric | NatType::SymmetricPortOnly)
    }
}

/// Generate a random 12-byte transaction ID
fn generate_transaction_id() -> [u8; 12] {
    let mut id = [0u8; 12];
    for byte in &mut id {
        *byte = rand::random();
    }
    id
}

/// Build a STUN binding request
fn build_binding_request(transaction_id: &[u8; 12]) -> Vec<u8> {
    let mut request = Vec::with_capacity(STUN_HEADER_SIZE);

    // Message type: Binding Request
    request.extend_from_slice(&STUN_BINDING_REQUEST.to_be_bytes());

    // Message length: 0 (no attributes)
    request.extend_from_slice(&0u16.to_be_bytes());

    // Magic cookie
    request.extend_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());

    // Transaction ID
    request.extend_from_slice(transaction_id);

    request
}

/// Parse a STUN binding response and extract the mapped address
fn parse_binding_response(data: &[u8], expected_txn_id: &[u8; 12]) -> Option<SocketAddr> {
    if data.len() < STUN_HEADER_SIZE {
        return None;
    }

    // Check message type
    let msg_type = u16::from_be_bytes([data[0], data[1]]);
    if msg_type != STUN_BINDING_RESPONSE {
        return None;
    }

    // Check magic cookie
    let cookie = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    if cookie != STUN_MAGIC_COOKIE {
        return None;
    }

    // Check transaction ID
    if &data[8..20] != expected_txn_id {
        return None;
    }

    // Parse message length
    let msg_len = u16::from_be_bytes([data[2], data[3]]) as usize;
    if data.len() < STUN_HEADER_SIZE + msg_len {
        return None;
    }

    // Parse attributes
    let mut offset = STUN_HEADER_SIZE;
    while offset + 4 <= STUN_HEADER_SIZE + msg_len {
        let attr_type = u16::from_be_bytes([data[offset], data[offset + 1]]);
        let attr_len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
        offset += 4;

        if offset + attr_len > data.len() {
            break;
        }

        match attr_type {
            STUN_ATTR_XOR_MAPPED_ADDRESS => {
                return parse_xor_mapped_address(&data[offset..offset + attr_len]);
            }
            STUN_ATTR_MAPPED_ADDRESS => {
                return parse_mapped_address(&data[offset..offset + attr_len]);
            }
            _ => {}
        }

        // Align to 4 bytes
        offset += (attr_len + 3) & !3;
    }

    None
}

/// Parse XOR-MAPPED-ADDRESS attribute
fn parse_xor_mapped_address(data: &[u8]) -> Option<SocketAddr> {
    if data.len() < 8 {
        return None;
    }

    let family = data[1];
    let xor_port = u16::from_be_bytes([data[2], data[3]]);
    let port = xor_port ^ ((STUN_MAGIC_COOKIE >> 16) as u16);

    match family {
        0x01 => {
            // IPv4
            if data.len() < 8 {
                return None;
            }
            let xor_addr = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
            let addr = xor_addr ^ STUN_MAGIC_COOKIE;
            let ip = std::net::Ipv4Addr::from(addr);
            Some(SocketAddr::new(ip.into(), port))
        }
        0x02 => {
            // IPv6
            if data.len() < 20 {
                return None;
            }
            // For IPv6, XOR with magic cookie + transaction ID
            // Simplified: just return None for now
            None
        }
        _ => None,
    }
}

/// Parse MAPPED-ADDRESS attribute (legacy)
fn parse_mapped_address(data: &[u8]) -> Option<SocketAddr> {
    if data.len() < 8 {
        return None;
    }

    let family = data[1];
    let port = u16::from_be_bytes([data[2], data[3]]);

    match family {
        0x01 => {
            // IPv4
            let ip = std::net::Ipv4Addr::new(data[4], data[5], data[6], data[7]);
            Some(SocketAddr::new(ip.into(), port))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_binding_request() {
        let txn_id = [1u8; 12];
        let request = build_binding_request(&txn_id);

        assert_eq!(request.len(), STUN_HEADER_SIZE);
        assert_eq!(request[0], 0x00);
        assert_eq!(request[1], 0x01); // Binding Request
        assert_eq!(request[2], 0x00);
        assert_eq!(request[3], 0x00); // Length 0
    }

    #[test]
    fn test_nat_type_traversable() {
        assert!(NatType::NoNat.stun_traversable());
        assert!(NatType::FullCone.stun_traversable());
        assert!(!NatType::Symmetric.stun_traversable());
        assert!(NatType::Symmetric.requires_turn());
    }

    #[test]
    fn test_generate_transaction_id() {
        let id1 = generate_transaction_id();
        let id2 = generate_transaction_id();
        assert_ne!(id1, id2);
    }

    // Integration test - requires network access
    #[tokio::test]
    #[ignore] // Run with: cargo test -- --ignored
    async fn test_stun_discover() {
        let client = StunClient::new();
        let result = client.discover("stun.l.google.com:19302").await;

        match result {
            Ok(r) => {
                println!("Mapped address: {}", r.mapped_address);
                println!("Local address: {}", r.local_address);
                println!("RTT: {:?}", r.rtt);
            }
            Err(e) => {
                println!("STUN failed (expected in some environments): {}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_nat_type_detection() {
        let client = StunClient::new();
        let nat_type = client.detect_nat_type().await;

        match nat_type {
            Ok(t) => {
                println!("NAT type: {:?}", t);
                println!("STUN traversable: {}", t.stun_traversable());
                println!("Requires TURN: {}", t.requires_turn());
            }
            Err(e) => {
                println!("NAT detection failed: {}", e);
            }
        }
    }
}
