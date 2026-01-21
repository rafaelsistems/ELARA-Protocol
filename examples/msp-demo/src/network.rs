//! Network utilities for MSP demo

use std::net::SocketAddr;

/// Parse a socket address from string
pub fn parse_addr(s: &str) -> Option<SocketAddr> {
    s.parse().ok()
}

/// Get local IP address (simplified)
pub fn local_ip() -> String {
    "127.0.0.1".to_string()
}
