//! Predefined load test scenarios
//!
//! This module provides predefined load test scenarios for common deployment sizes:
//! - Small deployment: 10 nodes
//! - Medium deployment: 100 nodes
//! - Large deployment: 1000 nodes

use std::time::Duration;
use crate::LoadTestConfig;

/// Small deployment scenario with 10 nodes
///
/// This scenario simulates a small ELARA Protocol deployment suitable for:
/// - Development and testing environments
/// - Small team collaborations
/// - Initial performance validation
///
/// Configuration:
/// - 10 nodes
/// - 5 connections per node
/// - 100 messages/second
/// - 60 second test duration
/// - 10 second ramp-up
pub fn small_deployment() -> LoadTestConfig {
    LoadTestConfig {
        num_nodes: 10,
        num_connections_per_node: 5,
        message_rate_per_second: 100,
        test_duration: Duration::from_secs(60),
        ramp_up_duration: Duration::from_secs(10),
    }
}

/// Medium deployment scenario with 100 nodes
///
/// This scenario simulates a medium-sized ELARA Protocol deployment suitable for:
/// - Production environments with moderate load
/// - Multi-team collaborations
/// - Performance baseline establishment
///
/// Configuration:
/// - 100 nodes
/// - 10 connections per node
/// - 1000 messages/second
/// - 300 second (5 minute) test duration
/// - 30 second ramp-up
pub fn medium_deployment() -> LoadTestConfig {
    LoadTestConfig {
        num_nodes: 100,
        num_connections_per_node: 10,
        message_rate_per_second: 1000,
        test_duration: Duration::from_secs(300),
        ramp_up_duration: Duration::from_secs(30),
    }
}

/// Large deployment scenario with 1000 nodes
///
/// This scenario simulates a large-scale ELARA Protocol deployment suitable for:
/// - High-scale production environments
/// - Stress testing and capacity planning
/// - Performance limits identification
///
/// Configuration:
/// - 1000 nodes
/// - 20 connections per node
/// - 10000 messages/second
/// - 600 second (10 minute) test duration
/// - 60 second ramp-up
pub fn large_deployment() -> LoadTestConfig {
    LoadTestConfig {
        num_nodes: 1000,
        num_connections_per_node: 20,
        message_rate_per_second: 10000,
        test_duration: Duration::from_secs(600),
        ramp_up_duration: Duration::from_secs(60),
    }
}

/// Custom scenario builder for specialized testing needs
pub struct ScenarioBuilder {
    config: LoadTestConfig,
}

impl ScenarioBuilder {
    /// Create a new scenario builder with default values
    pub fn new() -> Self {
        Self {
            config: small_deployment(),
        }
    }
    
    /// Set the number of nodes
    pub fn with_nodes(mut self, num_nodes: usize) -> Self {
        self.config.num_nodes = num_nodes;
        self
    }
    
    /// Set the number of connections per node
    pub fn with_connections_per_node(mut self, num_connections: usize) -> Self {
        self.config.num_connections_per_node = num_connections;
        self
    }
    
    /// Set the message rate per second
    pub fn with_message_rate(mut self, rate: usize) -> Self {
        self.config.message_rate_per_second = rate;
        self
    }
    
    /// Set the test duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.config.test_duration = duration;
        self
    }
    
    /// Set the ramp-up duration
    pub fn with_ramp_up(mut self, duration: Duration) -> Self {
        self.config.ramp_up_duration = duration;
        self
    }
    
    /// Build the configuration
    pub fn build(self) -> LoadTestConfig {
        self.config
    }
}

impl Default for ScenarioBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_deployment_config() {
        let config = small_deployment();
        assert_eq!(config.num_nodes, 10);
        assert_eq!(config.num_connections_per_node, 5);
        assert_eq!(config.message_rate_per_second, 100);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_medium_deployment_config() {
        let config = medium_deployment();
        assert_eq!(config.num_nodes, 100);
        assert_eq!(config.num_connections_per_node, 10);
        assert_eq!(config.message_rate_per_second, 1000);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_large_deployment_config() {
        let config = large_deployment();
        assert_eq!(config.num_nodes, 1000);
        assert_eq!(config.num_connections_per_node, 20);
        assert_eq!(config.message_rate_per_second, 10000);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_scenario_builder() {
        let config = ScenarioBuilder::new()
            .with_nodes(50)
            .with_connections_per_node(8)
            .with_message_rate(500)
            .with_duration(Duration::from_secs(120))
            .with_ramp_up(Duration::from_secs(20))
            .build();
        
        assert_eq!(config.num_nodes, 50);
        assert_eq!(config.num_connections_per_node, 8);
        assert_eq!(config.message_rate_per_second, 500);
        assert_eq!(config.test_duration, Duration::from_secs(120));
        assert_eq!(config.ramp_up_duration, Duration::from_secs(20));
        assert!(config.validate().is_ok());
    }
}
