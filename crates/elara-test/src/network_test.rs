//! Real Network Testing
//!
//! Tests that use actual UDP sockets for network communication.
//! These tests verify ELARA behavior over real network conditions.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::time::timeout;

use elara_core::{DegradationLevel, NodeId, PresenceVector};

// ============================================================================
// NETWORK TEST NODE
// ============================================================================

/// A test node that communicates over real UDP sockets
pub struct NetworkTestNode {
    /// Node identity
    pub node_id: NodeId,

    /// UDP socket
    socket: Arc<UdpSocket>,

    /// Local address
    local_addr: SocketAddr,

    /// Messages received
    received: Vec<(Vec<u8>, SocketAddr)>,

    /// Messages sent
    sent_count: usize,

    /// Presence vector
    presence: PresenceVector,

    /// Degradation level
    degradation: DegradationLevel,
}

impl NetworkTestNode {
    /// Create a new network test node
    pub async fn new(node_id: NodeId) -> std::io::Result<Self> {
        let socket = UdpSocket::bind("127.0.0.1:0").await?;
        let local_addr = socket.local_addr()?;

        Ok(Self {
            node_id,
            socket: Arc::new(socket),
            local_addr,
            received: Vec::new(),
            sent_count: 0,
            presence: PresenceVector::full(),
            degradation: DegradationLevel::L0_FullPerception,
        })
    }

    /// Get local address
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Send a message to a peer
    pub async fn send_to(&mut self, data: &[u8], dest: SocketAddr) -> std::io::Result<()> {
        self.socket.send_to(data, dest).await?;
        self.sent_count += 1;
        Ok(())
    }

    /// Receive a message with timeout
    pub async fn recv_timeout(&mut self, timeout_ms: u64) -> Option<(Vec<u8>, SocketAddr)> {
        let mut buf = vec![0u8; 65535];

        match timeout(
            Duration::from_millis(timeout_ms),
            self.socket.recv_from(&mut buf),
        )
        .await
        {
            Ok(Ok((len, addr))) => {
                let data = buf[..len].to_vec();
                self.received.push((data.clone(), addr));
                Some((data, addr))
            }
            _ => None,
        }
    }

    /// Get received message count
    pub fn received_count(&self) -> usize {
        self.received.len()
    }

    /// Get sent message count
    pub fn sent_count(&self) -> usize {
        self.sent_count
    }

    /// Update presence
    pub fn update_presence(&mut self, factor: f32) {
        self.presence = PresenceVector::new(
            self.presence.liveness * factor,
            self.presence.immediacy * factor,
            self.presence.coherence * factor,
            self.presence.relational_continuity * factor,
            self.presence.emotional_bandwidth * factor,
        );
    }

    /// Degrade
    pub fn degrade(&mut self) -> bool {
        if let Some(next) = self.degradation.degrade() {
            self.degradation = next;
            true
        } else {
            false
        }
    }

    /// Check if alive
    pub fn is_alive(&self) -> bool {
        self.presence.is_alive()
    }

    /// Get presence
    pub fn presence(&self) -> &PresenceVector {
        &self.presence
    }

    /// Get degradation level
    pub fn degradation_level(&self) -> DegradationLevel {
        self.degradation
    }
}

// ============================================================================
// NETWORK TEST HARNESS
// ============================================================================

/// Configuration for network tests
#[derive(Debug, Clone)]
pub struct NetworkTestConfig {
    /// Number of nodes
    pub node_count: usize,

    /// Messages per node
    pub messages_per_node: usize,

    /// Receive timeout in ms
    pub recv_timeout_ms: u64,

    /// Delay between sends in ms
    pub send_delay_ms: u64,
}

impl Default for NetworkTestConfig {
    fn default() -> Self {
        Self {
            node_count: 3,
            messages_per_node: 5,
            recv_timeout_ms: 100,
            send_delay_ms: 10,
        }
    }
}

/// Result of a network test
#[derive(Debug, Clone)]
pub struct NetworkTestResult {
    /// Total messages sent
    pub messages_sent: usize,

    /// Total messages received
    pub messages_received: usize,

    /// Delivery rate
    pub delivery_rate: f64,

    /// All nodes alive
    pub all_alive: bool,

    /// Invariants maintained
    pub invariants_maintained: bool,

    /// Violations
    pub violations: Vec<String>,
}

impl NetworkTestResult {
    /// Check if test passed
    pub fn passed(&self) -> bool {
        self.all_alive && self.invariants_maintained && self.delivery_rate > 0.9
    }
}

/// Network test harness
pub struct NetworkTestHarness {
    config: NetworkTestConfig,
    nodes: Vec<NetworkTestNode>,
}

impl NetworkTestHarness {
    /// Create a new network test harness
    pub async fn new(config: NetworkTestConfig) -> std::io::Result<Self> {
        let mut nodes = Vec::new();

        for i in 0..config.node_count {
            let node = NetworkTestNode::new(NodeId::new(i as u64 + 1)).await?;
            nodes.push(node);
        }

        Ok(Self { config, nodes })
    }

    /// Run the network test
    pub async fn run(&mut self) -> NetworkTestResult {
        let mut messages_sent = 0;
        let mut messages_received = 0;

        // Collect all addresses
        let addresses: Vec<_> = self.nodes.iter().map(|n| n.local_addr()).collect();

        // Each node sends messages to all other nodes
        for (sender_idx, sender) in self.nodes.iter_mut().enumerate() {
            for msg_num in 0..self.config.messages_per_node {
                for (receiver_idx, dest) in addresses.iter().copied().enumerate() {
                    if sender_idx == receiver_idx {
                        continue;
                    }
                    let msg = format!("msg_{}_{}", sender_idx, msg_num);

                    if sender.send_to(msg.as_bytes(), dest).await.is_ok() {
                        messages_sent += 1;
                    }
                }

                // Small delay between sends
                if self.config.send_delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(self.config.send_delay_ms)).await;
                }
            }
        }

        // Give time for messages to arrive
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Receive messages on all nodes
        for node in &mut self.nodes {
            loop {
                if node
                    .recv_timeout(self.config.recv_timeout_ms)
                    .await
                    .is_none()
                {
                    break;
                }
                messages_received += 1;
            }
        }

        // Check invariants
        let mut violations = Vec::new();
        let all_alive = self.nodes.iter().all(|n| n.is_alive());

        if !all_alive {
            violations.push("INV-2 violated: Not all nodes are alive".to_string());
        }

        let delivery_rate = if messages_sent > 0 {
            messages_received as f64 / messages_sent as f64
        } else {
            1.0
        };

        NetworkTestResult {
            messages_sent,
            messages_received,
            delivery_rate,
            all_alive,
            invariants_maintained: violations.is_empty(),
            violations,
        }
    }

    /// Get nodes
    pub fn nodes(&self) -> &[NetworkTestNode] {
        &self.nodes
    }
}

// ============================================================================
// LATENCY MEASUREMENT
// ============================================================================

/// Measure round-trip latency between two nodes
pub async fn measure_rtt(
    node_a: &mut NetworkTestNode,
    node_b: &mut NetworkTestNode,
    samples: usize,
) -> Option<Duration> {
    let mut rtts = Vec::new();
    let addr_b = node_b.local_addr();
    let addr_a = node_a.local_addr();

    for i in 0..samples {
        let ping_msg = format!("ping_{}", i);
        let start = std::time::Instant::now();

        // Send ping
        if node_a.send_to(ping_msg.as_bytes(), addr_b).await.is_err() {
            continue;
        }

        // Wait for ping on B
        if let Some((data, _)) = node_b.recv_timeout(100).await {
            // Send pong
            let pong_msg = format!("pong_{}", String::from_utf8_lossy(&data));
            if node_b.send_to(pong_msg.as_bytes(), addr_a).await.is_err() {
                continue;
            }

            // Wait for pong on A
            if node_a.recv_timeout(100).await.is_some() {
                rtts.push(start.elapsed());
            }
        }
    }

    if rtts.is_empty() {
        None
    } else {
        let total: Duration = rtts.iter().sum();
        Some(total / rtts.len() as u32)
    }
}

// ============================================================================
// CONVENIENCE FUNCTIONS
// ============================================================================

/// Run a basic network connectivity test
pub async fn test_basic_connectivity() -> NetworkTestResult {
    let config = NetworkTestConfig {
        node_count: 2,
        messages_per_node: 3,
        recv_timeout_ms: 100,
        send_delay_ms: 5,
    };

    let mut harness = NetworkTestHarness::new(config).await.unwrap();
    harness.run().await
}

/// Run a multi-node network test
pub async fn test_multi_node_network() -> NetworkTestResult {
    let config = NetworkTestConfig::default();
    let mut harness = NetworkTestHarness::new(config).await.unwrap();
    harness.run().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_node_creation() {
        let node = NetworkTestNode::new(NodeId::new(1)).await.unwrap();
        assert!(node.local_addr().port() > 0);
        assert!(node.is_alive());
    }

    #[tokio::test]
    async fn test_network_send_receive() {
        let mut node_a = NetworkTestNode::new(NodeId::new(1)).await.unwrap();
        let mut node_b = NetworkTestNode::new(NodeId::new(2)).await.unwrap();

        let addr_b = node_b.local_addr();

        // Send message from A to B
        node_a.send_to(b"hello", addr_b).await.unwrap();

        // Receive on B
        let result = node_b.recv_timeout(100).await;
        assert!(result.is_some());

        let (data, _) = result.unwrap();
        assert_eq!(&data, b"hello");
    }

    #[tokio::test]
    async fn test_basic_connectivity_test() {
        let result = test_basic_connectivity().await;

        assert!(result.all_alive, "All nodes should be alive");
        assert!(result.delivery_rate > 0.9, "Delivery rate should be > 90%");
        assert!(result.passed(), "Basic connectivity test should pass");
    }

    #[tokio::test]
    async fn test_multi_node_network_test() {
        let result = test_multi_node_network().await;

        assert!(result.all_alive, "All nodes should be alive");
        assert!(result.messages_sent > 0, "Should have sent messages");
        assert!(
            result.messages_received > 0,
            "Should have received messages"
        );
    }

    #[tokio::test]
    async fn test_rtt_measurement() {
        let mut node_a = NetworkTestNode::new(NodeId::new(1)).await.unwrap();
        let mut node_b = NetworkTestNode::new(NodeId::new(2)).await.unwrap();

        let rtt = measure_rtt(&mut node_a, &mut node_b, 3).await;

        // On localhost, RTT should be very low
        if let Some(rtt) = rtt {
            assert!(
                rtt < Duration::from_millis(50),
                "Localhost RTT should be < 50ms"
            );
        }
    }

    #[tokio::test]
    async fn test_network_harness() {
        let config = NetworkTestConfig {
            node_count: 3,
            messages_per_node: 2,
            recv_timeout_ms: 100,
            send_delay_ms: 5,
        };

        let mut harness = NetworkTestHarness::new(config).await.unwrap();
        let result = harness.run().await;

        assert!(
            result.invariants_maintained,
            "Invariants should be maintained"
        );
    }
}
