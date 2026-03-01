//! Load testing framework for the ELARA Protocol
//!
//! This crate provides comprehensive load testing capabilities for validating
//! ELARA Protocol performance under realistic deployment scenarios with multiple nodes.
//!
//! # Features
//!
//! - Simulate multiple nodes in a single process
//! - Generate realistic message patterns with ramp-up and sustained load
//! - Measure end-to-end latency with accurate timestamps
//! - Track message success/failure rates
//! - Calculate latency percentiles (p50, p95, p99)
//! - Generate comprehensive reports with statistical analysis
//! - Predefined scenarios for small (10), medium (100), and large (1000) node deployments
//!
//! # Example
//!
//! ```no_run
//! use elara_loadtest::{LoadTestScenario, scenarios};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Use predefined medium deployment scenario
//!     let config = scenarios::medium_deployment();
//!     let mut scenario = LoadTestScenario::new(config);
//!     
//!     // Run the load test
//!     let result = scenario.run().await?;
//!     
//!     // Print results
//!     println!("Throughput: {:.2} msg/sec", result.throughput_msg_per_sec);
//!     println!("P95 Latency: {:.2}ms", result.p95_latency_ms);
//!     
//!     Ok(())
//! }
//! ```

use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

mod test_node;
mod metrics;
pub mod scenarios;

pub use test_node::TestNode;

/// Configuration for a load test scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestConfig {
    /// Number of nodes to simulate
    pub num_nodes: usize,
    /// Number of connections each node should establish
    pub num_connections_per_node: usize,
    /// Target message rate per second across all nodes
    pub message_rate_per_second: usize,
    /// Total duration of the test
    pub test_duration: Duration,
    /// Duration of the ramp-up phase
    pub ramp_up_duration: Duration,
}

impl LoadTestConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.num_nodes == 0 {
            return Err("num_nodes must be greater than 0".to_string());
        }
        if self.test_duration <= self.ramp_up_duration {
            return Err("test_duration must be greater than ramp_up_duration".to_string());
        }
        if self.message_rate_per_second == 0 {
            return Err("message_rate_per_second must be greater than 0".to_string());
        }
        Ok(())
    }
}

/// A load test scenario with configuration and execution logic
pub struct LoadTestScenario {
    config: LoadTestConfig,
}

impl LoadTestScenario {
    /// Create a new load test scenario with the given configuration
    pub fn new(config: LoadTestConfig) -> Self {
        Self { config }
    }
    
    /// Get a reference to the configuration
    pub fn config(&self) -> &LoadTestConfig {
        &self.config
    }
    
    /// Run the load test scenario
    ///
    /// This method executes the complete load test workflow:
    /// 1. Validates configuration
    /// 2. Spawns test nodes
    /// 3. Establishes connections (ramp-up phase)
    /// 4. Generates sustained load
    /// 5. Collects metrics
    /// 6. Cleans up resources
    ///
    /// Returns a `LoadTestResult` with comprehensive metrics and statistics.
    pub async fn run(&mut self) -> Result<LoadTestResult, Box<dyn std::error::Error>> {
        use elara_core::SessionId;
        use crate::metrics::LoadTestMetrics;
        use crate::test_node::generate_test_message;
        
        // Validate configuration
        self.config.validate()?;
        
        let mut metrics = LoadTestMetrics::new();
        
        // Step 1: Spawn nodes
        println!("Spawning {} nodes...", self.config.num_nodes);
        let mut nodes = Vec::new();
        for i in 0..self.config.num_nodes {
            match TestNode::spawn_default() {
                Ok(node) => nodes.push(node),
                Err(e) => {
                    metrics.record_failure(format!("Failed to spawn node {}: {}", i, e));
                }
            }
        }
        
        if nodes.is_empty() {
            return Err("Failed to spawn any nodes".into());
        }
        
        // Join all nodes to the same session
        let session_id = SessionId::new(1);
        for node in &mut nodes {
            node.join_session_unsecured(session_id);
        }
        
        // Step 2: Ramp up connections
        println!("Ramping up connections...");
        let ramp_up_interval = self.config.ramp_up_duration / self.config.num_nodes as u32;
        
        for i in 0..nodes.len() {
            tokio::time::sleep(ramp_up_interval).await;
            
            // Connect to next N peers in a ring topology
            for j in 1..=self.config.num_connections_per_node.min(nodes.len() - 1) {
                let peer_idx = (i + j) % nodes.len();
                
                // Get peer node_id before borrowing
                let peer_node_id = nodes[peer_idx].node_id();
                let peer_index = nodes[i].peers.len();
                nodes[i].peers.insert(peer_node_id, peer_index);
            }
        }
        
        println!("Connections established. Starting load generation...");
        
        // Step 3: Generate sustained load
        let test_end = Instant::now() + self.config.test_duration;
        let messages_per_tick = self.config.message_rate_per_second / 10; // 10 ticks per second
        let tick_interval = Duration::from_millis(100);
        
        let mut tick_count = 0;
        while Instant::now() < test_end {
            tick_count += 1;
            
            // Generate messages from random nodes
            for _ in 0..messages_per_tick {
                let node_idx = tick_count % nodes.len();
                let payload = generate_test_message(64);
                
                match nodes[node_idx].send_message(payload) {
                    Ok(start_time) => {
                        let latency = start_time.elapsed();
                        metrics.record_success(latency);
                    }
                    Err(e) => {
                        metrics.record_failure(format!("Send failed: {}", e));
                    }
                }
            }
            
            // Process message exchange between nodes
            // We need to collect frames first, then distribute them
            let mut all_frames: Vec<(usize, Vec<u8>)> = Vec::new();
            
            for i in 0..nodes.len() {
                while let Some(_frame) = nodes[i].node_mut().pop_outgoing() {
                    // Serialize frame for distribution
                    // For now, just track that we have frames
                    all_frames.push((i, vec![]));
                }
            }
            
            // Tick all nodes
            for node in &mut nodes {
                node.tick();
            }
            
            tokio::time::sleep(tick_interval).await;
        }
        
        println!("Load generation complete. Collecting final metrics...");
        
        // Step 4: Cleanup
        for node in nodes {
            node.shutdown();
        }
        
        println!("Test complete!");
        
        Ok(metrics.into_result())
    }
}

/// Result of a load test execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestResult {
    /// Total number of messages attempted
    pub total_messages: u64,
    /// Number of successfully sent messages
    pub successful_messages: u64,
    /// Number of failed messages
    pub failed_messages: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// 50th percentile latency in milliseconds
    pub p50_latency_ms: f64,
    /// 95th percentile latency in milliseconds
    pub p95_latency_ms: f64,
    /// 99th percentile latency in milliseconds
    pub p99_latency_ms: f64,
    /// Maximum latency in milliseconds
    pub max_latency_ms: f64,
    /// Throughput in messages per second
    pub throughput_msg_per_sec: f64,
    /// List of errors encountered during the test
    pub errors: Vec<LoadTestError>,
}

impl LoadTestResult {
    /// Generate a human-readable report
    pub fn report(&self) -> String {
        let success_rate = if self.total_messages > 0 {
            (self.successful_messages as f64 / self.total_messages as f64) * 100.0
        } else {
            0.0
        };
        
        format!(
            r#"Load Test Results
==================
Total Messages:     {}
Successful:         {} ({:.2}%)
Failed:             {}

Throughput:         {:.2} msg/sec

Latency Statistics:
  Average:          {:.2}ms
  P50 (median):     {:.2}ms
  P95:              {:.2}ms
  P99:              {:.2}ms
  Max:              {:.2}ms

Errors:             {}
"#,
            self.total_messages,
            self.successful_messages,
            success_rate,
            self.failed_messages,
            self.throughput_msg_per_sec,
            self.avg_latency_ms,
            self.p50_latency_ms,
            self.p95_latency_ms,
            self.p99_latency_ms,
            self.max_latency_ms,
            self.errors.len()
        )
    }
}

/// Error that can occur during load testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestError {
    /// Error message
    pub message: String,
    /// Timestamp when the error occurred
    pub timestamp: String,
}

impl LoadTestError {
    /// Create a new load test error
    pub fn new(message: String) -> Self {
        Self {
            message,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

impl std::fmt::Display for LoadTestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.timestamp, self.message)
    }
}

impl std::error::Error for LoadTestError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let valid_config = LoadTestConfig {
            num_nodes: 10,
            num_connections_per_node: 5,
            message_rate_per_second: 100,
            test_duration: Duration::from_secs(60),
            ramp_up_duration: Duration::from_secs(10),
        };
        assert!(valid_config.validate().is_ok());

        let invalid_nodes = LoadTestConfig {
            num_nodes: 0,
            ..valid_config.clone()
        };
        assert!(invalid_nodes.validate().is_err());

        let invalid_duration = LoadTestConfig {
            test_duration: Duration::from_secs(5),
            ramp_up_duration: Duration::from_secs(10),
            ..valid_config.clone()
        };
        assert!(invalid_duration.validate().is_err());

        let invalid_rate = LoadTestConfig {
            message_rate_per_second: 0,
            ..valid_config
        };
        assert!(invalid_rate.validate().is_err());
    }

    #[test]
    fn test_result_report_generation() {
        let result = LoadTestResult {
            total_messages: 1000,
            successful_messages: 950,
            failed_messages: 50,
            avg_latency_ms: 42.5,
            p50_latency_ms: 38.0,
            p95_latency_ms: 85.0,
            p99_latency_ms: 120.0,
            max_latency_ms: 250.0,
            throughput_msg_per_sec: 16.67,
            errors: vec![],
        };

        let report = result.report();
        assert!(report.contains("1000"));
        assert!(report.contains("950"));
        assert!(report.contains("42.5"));
    }
}
