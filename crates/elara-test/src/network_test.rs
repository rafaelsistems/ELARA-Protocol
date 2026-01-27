//! Real Network Testing
//!
//! Tests that use actual UDP sockets for network communication.
//! These tests verify ELARA behavior over real network conditions.

use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::net::UdpSocket;
use tokio::time::timeout;

use elara_core::{DegradationLevel, NodeId, PresenceVector};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

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

    recv_buf: Vec<u8>,

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
            recv_buf: vec![0u8; 65535],
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
        match timeout(
            Duration::from_millis(timeout_ms),
            self.socket.recv_from(&mut self.recv_buf),
        )
        .await
        {
            Ok(Ok((len, addr))) => {
                let data = self.recv_buf[..len].to_vec();
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

    pub loss_rate: f32,

    pub jitter_ms: u64,

    pub rng_seed: u64,

    pub nat_relay: bool,
}

impl Default for NetworkTestConfig {
    fn default() -> Self {
        Self {
            node_count: 3,
            messages_per_node: 5,
            recv_timeout_ms: 100,
            send_delay_ms: 10,
            loss_rate: 0.0,
            jitter_ms: 0,
            rng_seed: 42,
            nat_relay: false,
        }
    }
}

impl NetworkTestConfig {
    fn validate(&self) -> std::io::Result<()> {
        if self.node_count == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "node_count must be greater than 0",
            ));
        }
        if self.messages_per_node == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "messages_per_node must be greater than 0",
            ));
        }
        if self.recv_timeout_ms == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "recv_timeout_ms must be greater than 0",
            ));
        }
        if !self.loss_rate.is_finite() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "loss_rate must be finite",
            ));
        }
        Ok(())
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

    pub fn failure(violations: Vec<String>) -> Self {
        Self {
            messages_sent: 0,
            messages_received: 0,
            delivery_rate: 0.0,
            all_alive: false,
            invariants_maintained: false,
            violations,
        }
    }
}

/// Network test harness
pub struct NetworkTestHarness {
    config: NetworkTestConfig,
    nodes: Vec<NetworkTestNode>,
}

struct NatRelay {
    addr: SocketAddr,
    handle: tokio::task::JoinHandle<()>,
}

impl NatRelay {
    async fn start(routes: Vec<SocketAddr>) -> std::io::Result<Self> {
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await?);
        let addr = socket.local_addr()?;
        let routes = Arc::new(routes);
        let handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 65535];
            loop {
                let (len, _) = match socket.recv_from(&mut buf).await {
                    Ok(result) => result,
                    Err(_) => break,
                };
                let Some((dest_idx, payload_start)) = parse_nat_payload(&buf[..len]) else {
                    continue;
                };
                if let Some(dest) = routes.get(dest_idx) {
                    let _ = socket.send_to(&buf[payload_start..len], dest).await;
                }
            }
        });

        Ok(Self { addr, handle })
    }

    async fn shutdown(self) {
        self.handle.abort();
        let _ = self.handle.await;
    }
}

fn parse_nat_payload(buf: &[u8]) -> Option<(usize, usize)> {
    if buf.len() < 2 {
        return None;
    }
    let dest = u16::from_le_bytes([buf[0], buf[1]]) as usize;
    Some((dest, 2))
}

impl NetworkTestHarness {
    /// Create a new network test harness
    pub async fn new(config: NetworkTestConfig) -> std::io::Result<Self> {
        config.validate()?;
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
        let mut rng = StdRng::seed_from_u64(self.config.rng_seed);
        let mut violations = Vec::new();

        if self.nodes.len() < 2 {
            violations.push("INV-0 violated: Insufficient nodes".to_string());
        }

        // Collect all addresses
        let addresses: Vec<_> = self.nodes.iter().map(|n| n.local_addr()).collect();

        let mut relay = None;
        let mut relay_addr = None;
        if self.config.nat_relay {
            match NatRelay::start(addresses.clone()).await {
                Ok(nat) => {
                    relay_addr = Some(nat.addr);
                    relay = Some(nat);
                }
                Err(err) => {
                    violations.push(format!("NAT relay failed: {}", err));
                }
            }
        }

        let loss_rate = self.config.loss_rate.clamp(0.0, 1.0);

        // Each node sends messages to all other nodes
        let mut send_failures = 0usize;
        for (sender_idx, sender) in self.nodes.iter_mut().enumerate() {
            for msg_num in 0..self.config.messages_per_node {
                for (receiver_idx, dest) in addresses.iter().copied().enumerate() {
                    if sender_idx == receiver_idx {
                        continue;
                    }
                    let msg_bytes: Vec<u8> = if self.config.nat_relay {
                        let payload = format!("msg_{}_{}", sender_idx, msg_num).into_bytes();
                        let mut buf = Vec::with_capacity(2 + payload.len());
                        let idx_le = (receiver_idx as u16).to_le_bytes();
                        buf.extend_from_slice(&idx_le);
                        buf.extend_from_slice(&payload);
                        buf
                    } else {
                        format!("msg_{}_{}", sender_idx, msg_num).into_bytes()
                    };
                    messages_sent += 1;

                    if loss_rate > 0.0 && rng.gen::<f32>() < loss_rate {
                        continue;
                    }

                    if self.config.jitter_ms > 0 {
                        let jitter = rng.gen_range(0..=self.config.jitter_ms);
                        tokio::time::sleep(Duration::from_millis(jitter)).await;
                    }

                    let target = relay_addr.unwrap_or(dest);
                    if sender.send_to(&msg_bytes, target).await.is_err() {
                        send_failures += 1;
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

        if let Some(relay) = relay {
            relay.shutdown().await;
        }

        let all_alive = self.nodes.iter().all(|n| n.is_alive());

        if !all_alive {
            violations.push("INV-2 violated: Not all nodes are alive".to_string());
        }

        if send_failures > 0 {
            violations.push(format!("Send failures: {}", send_failures));
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
    let rtts = measure_rtt_samples(node_a, node_b, samples).await;
    if rtts.is_empty() {
        None
    } else {
        let total: Duration = rtts.iter().sum();
        Some(total / rtts.len() as u32)
    }
}

pub async fn measure_rtt_samples(
    node_a: &mut NetworkTestNode,
    node_b: &mut NetworkTestNode,
    samples: usize,
) -> Vec<Duration> {
    let mut rtts = Vec::new();
    let addr_b = node_b.local_addr();
    let addr_a = node_a.local_addr();

    for i in 0..samples {
        let ping_msg = format!("ping_{}", i);
        let start = std::time::Instant::now();

        if node_a.send_to(ping_msg.as_bytes(), addr_b).await.is_err() {
            continue;
        }

        if let Some((data, _)) = node_b.recv_timeout(100).await {
            let pong_msg = format!("pong_{}", String::from_utf8_lossy(&data));
            if node_b.send_to(pong_msg.as_bytes(), addr_a).await.is_err() {
                continue;
            }

            if node_a.recv_timeout(100).await.is_some() {
                rtts.push(start.elapsed());
            }
        }
    }

    rtts
}

pub async fn measure_rtt_samples_with_conditions(
    node_a: &mut NetworkTestNode,
    node_b: &mut NetworkTestNode,
    samples: usize,
    loss_rate: f32,
    jitter_ms: u64,
    rng_seed: u64,
) -> Vec<Duration> {
    let mut rtts = Vec::new();
    let addr_b = node_b.local_addr();
    let addr_a = node_a.local_addr();
    let mut rng = StdRng::seed_from_u64(rng_seed);
    let clamped_loss = loss_rate.clamp(0.0, 1.0);

    for i in 0..samples {
        if clamped_loss > 0.0 && rng.gen::<f32>() < clamped_loss {
            continue;
        }

        if jitter_ms > 0 {
            let jitter = rng.gen_range(0..=jitter_ms);
            tokio::time::sleep(Duration::from_millis(jitter)).await;
        }

        let ping_msg = format!("ping_{}", i);
        let start = Instant::now();

        if node_a.send_to(ping_msg.as_bytes(), addr_b).await.is_err() {
            continue;
        }

        if let Some((data, _)) = node_b.recv_timeout(100).await {
            if clamped_loss > 0.0 && rng.gen::<f32>() < clamped_loss {
                continue;
            }

            if jitter_ms > 0 {
                let jitter = rng.gen_range(0..=jitter_ms);
                tokio::time::sleep(Duration::from_millis(jitter)).await;
            }

            let pong_msg = format!("pong_{}", String::from_utf8_lossy(&data));
            if node_b.send_to(pong_msg.as_bytes(), addr_a).await.is_err() {
                continue;
            }

            if node_a.recv_timeout(100).await.is_some() {
                rtts.push(start.elapsed());
            }
        }
    }

    rtts
}

pub async fn measure_rtt_nat(samples: usize) -> Option<Duration> {
    let rtts = measure_rtt_nat_samples(samples).await;
    if rtts.is_empty() {
        None
    } else {
        let total: Duration = rtts.iter().sum();
        Some(total / rtts.len() as u32)
    }
}

pub async fn measure_rtt_nat_samples(samples: usize) -> Vec<Duration> {
    let mut node_a = match NetworkTestNode::new(NodeId::new(1)).await {
        Ok(node) => node,
        Err(_) => return Vec::new(),
    };
    let mut node_b = match NetworkTestNode::new(NodeId::new(2)).await {
        Ok(node) => node,
        Err(_) => return Vec::new(),
    };
    let routes = vec![node_a.local_addr(), node_b.local_addr()];
    let relay = match NatRelay::start(routes).await {
        Ok(relay) => relay,
        Err(_) => return Vec::new(),
    };
    let relay_addr = relay.addr;
    let mut rtts = Vec::new();

    for i in 0..samples {
        let payload = format!("ping_{}", i).into_bytes();
        let mut buf = Vec::with_capacity(2 + payload.len());
        buf.extend_from_slice(&1u16.to_le_bytes());
        buf.extend_from_slice(&payload);
        let start = Instant::now();

        if node_a.send_to(&buf, relay_addr).await.is_err() {
            continue;
        }

        if let Some((data, _)) = node_b.recv_timeout(100).await {
            let pong_payload = format!("pong_{}", String::from_utf8_lossy(&data)).into_bytes();
            let mut pong_buf = Vec::with_capacity(2 + pong_payload.len());
            pong_buf.extend_from_slice(&0u16.to_le_bytes());
            pong_buf.extend_from_slice(&pong_payload);
            if node_b.send_to(&pong_buf, relay_addr).await.is_err() {
                continue;
            }

            if node_a.recv_timeout(100).await.is_some() {
                rtts.push(start.elapsed());
            }
        }
    }

    relay.shutdown().await;

    rtts
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
        loss_rate: 0.0,
        jitter_ms: 0,
        rng_seed: 1,
        nat_relay: false,
    };

    match NetworkTestHarness::new(config).await {
        Ok(mut harness) => harness.run().await,
        Err(err) => {
            NetworkTestResult::failure(vec![format!("network harness creation failed: {}", err)])
        }
    }
}

/// Run a multi-node network test
pub async fn test_multi_node_network() -> NetworkTestResult {
    let config = NetworkTestConfig::default();
    match NetworkTestHarness::new(config).await {
        Ok(mut harness) => harness.run().await,
        Err(err) => {
            NetworkTestResult::failure(vec![format!("network harness creation failed: {}", err)])
        }
    }
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
    async fn test_invalid_config_rejected() {
        let config = NetworkTestConfig {
            node_count: 0,
            messages_per_node: 1,
            recv_timeout_ms: 100,
            send_delay_ms: 0,
            loss_rate: 0.0,
            jitter_ms: 0,
            rng_seed: 1,
            nat_relay: false,
        };

        let result = NetworkTestHarness::new(config).await;
        assert!(result.is_err());
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
            let max_rtt_ms = if cfg!(target_os = "windows") {
                500
            } else {
                200
            };
            assert!(
                rtt < Duration::from_millis(max_rtt_ms),
                "Localhost RTT should be < {max_rtt_ms}ms"
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
            loss_rate: 0.0,
            jitter_ms: 0,
            rng_seed: 5,
            nat_relay: false,
        };

        let mut harness = NetworkTestHarness::new(config).await.unwrap();
        let result = harness.run().await;

        assert!(
            result.invariants_maintained,
            "Invariants should be maintained"
        );
    }

    #[tokio::test]
    async fn test_network_loss_and_jitter() {
        let config = NetworkTestConfig {
            node_count: 3,
            messages_per_node: 50,
            recv_timeout_ms: 150,
            send_delay_ms: 0,
            loss_rate: 0.2,
            jitter_ms: 25,
            rng_seed: 9,
            nat_relay: false,
        };

        let mut harness = NetworkTestHarness::new(config).await.unwrap();
        let result = harness.run().await;

        assert!(result.all_alive, "All nodes should be alive");
        assert!(result.delivery_rate < 0.95, "Delivery rate should drop");
    }

    #[tokio::test]
    async fn test_network_nat_relay() {
        let config = NetworkTestConfig {
            node_count: 3,
            messages_per_node: 5,
            recv_timeout_ms: 150,
            send_delay_ms: 5,
            loss_rate: 0.0,
            jitter_ms: 0,
            rng_seed: 11,
            nat_relay: true,
        };

        let mut harness = NetworkTestHarness::new(config).await.unwrap();
        let result = harness.run().await;

        assert!(result.all_alive, "All nodes should be alive");
        assert!(result.delivery_rate > 0.9, "Delivery rate should be high");
    }

    #[tokio::test]
    async fn test_nat_relay_routes_to_destination() {
        let mut node_a = NetworkTestNode::new(NodeId::new(1)).await.unwrap();
        let mut node_b = NetworkTestNode::new(NodeId::new(2)).await.unwrap();

        let relay = NatRelay::start(vec![node_a.local_addr(), node_b.local_addr()])
            .await
            .unwrap();

        let mut msg = Vec::with_capacity(2 + 5);
        msg.extend_from_slice(&(1u16).to_le_bytes());
        msg.extend_from_slice(b"hello");
        node_a.send_to(&msg, relay.addr).await.unwrap();

        let received_b = node_b.recv_timeout(100).await;
        assert!(received_b.is_some());
        let (data, _) = received_b.unwrap();
        assert_eq!(&data, b"hello");

        let received_a = node_a.recv_timeout(50).await;
        assert!(received_a.is_none());

        relay.shutdown().await;
    }

    #[tokio::test]
    async fn test_network_high_loss_and_jitter() {
        let config = NetworkTestConfig {
            node_count: 3,
            messages_per_node: 20,
            recv_timeout_ms: 200,
            send_delay_ms: 0,
            loss_rate: 0.8,
            jitter_ms: 100,
            rng_seed: 17,
            nat_relay: false,
        };

        let mut harness = NetworkTestHarness::new(config).await.unwrap();
        let result = harness.run().await;

        assert!(result.all_alive, "All nodes should be alive");
        assert!(result.delivery_rate < 0.5, "Delivery rate should be low");
    }
}
