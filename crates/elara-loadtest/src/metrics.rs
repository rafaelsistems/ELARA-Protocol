//! Metrics collection for load testing
//!
//! This module provides the `LoadTestMetrics` struct for collecting and analyzing
//! performance metrics during load test execution.

use std::time::Duration;
use crate::{LoadTestResult, LoadTestError};

/// Metrics collector for load test execution
pub struct LoadTestMetrics {
    /// Total messages attempted
    pub total_messages: u64,
    /// Successfully sent messages
    pub successful_messages: u64,
    /// Failed messages
    pub failed_messages: u64,
    /// Latency measurements in milliseconds
    pub latencies: Vec<f64>,
    /// Errors encountered
    pub errors: Vec<LoadTestError>,
    /// Test start time
    pub start_time: std::time::Instant,
}

impl LoadTestMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            total_messages: 0,
            successful_messages: 0,
            failed_messages: 0,
            latencies: Vec::new(),
            errors: Vec::new(),
            start_time: std::time::Instant::now(),
        }
    }
    
    /// Record a successful message send with latency
    pub fn record_success(&mut self, latency: Duration) {
        self.total_messages += 1;
        self.successful_messages += 1;
        self.latencies.push(latency.as_secs_f64() * 1000.0);
    }
    
    /// Record a failed message send
    pub fn record_failure(&mut self, error: String) {
        self.total_messages += 1;
        self.failed_messages += 1;
        self.errors.push(LoadTestError::new(error));
    }
    
    /// Calculate average latency in milliseconds
    pub fn avg_latency_ms(&self) -> f64 {
        if self.latencies.is_empty() {
            return 0.0;
        }
        self.latencies.iter().sum::<f64>() / self.latencies.len() as f64
    }
    
    /// Calculate a specific percentile latency
    pub fn percentile(&self, p: f64) -> f64 {
        if self.latencies.is_empty() {
            return 0.0;
        }
        
        let mut sorted = self.latencies.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        let index = ((p * sorted.len() as f64) as usize).min(sorted.len() - 1);
        sorted[index]
    }
    
    /// Get maximum latency
    pub fn max_latency_ms(&self) -> f64 {
        self.latencies.iter()
            .cloned()
            .fold(0.0, f64::max)
    }
    
    /// Calculate throughput in messages per second
    pub fn throughput(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed == 0.0 {
            return 0.0;
        }
        self.successful_messages as f64 / elapsed
    }
    
    /// Convert metrics to a LoadTestResult
    pub fn into_result(self) -> LoadTestResult {
        LoadTestResult {
            total_messages: self.total_messages,
            successful_messages: self.successful_messages,
            failed_messages: self.failed_messages,
            avg_latency_ms: self.avg_latency_ms(),
            p50_latency_ms: self.percentile(0.50),
            p95_latency_ms: self.percentile(0.95),
            p99_latency_ms: self.percentile(0.99),
            max_latency_ms: self.max_latency_ms(),
            throughput_msg_per_sec: self.throughput(),
            errors: self.errors,
        }
    }
}

impl Default for LoadTestMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        let mut metrics = LoadTestMetrics::new();
        
        metrics.record_success(Duration::from_millis(10));
        metrics.record_success(Duration::from_millis(20));
        metrics.record_failure("test error".to_string());
        
        assert_eq!(metrics.total_messages, 3);
        assert_eq!(metrics.successful_messages, 2);
        assert_eq!(metrics.failed_messages, 1);
        assert_eq!(metrics.latencies.len(), 2);
        assert_eq!(metrics.errors.len(), 1);
    }

    #[test]
    fn test_percentile_calculation() {
        let mut metrics = LoadTestMetrics::new();
        
        // Add latencies: 10, 20, 30, 40, 50 ms
        for i in 1..=5 {
            metrics.record_success(Duration::from_millis(i * 10));
        }
        
        assert_eq!(metrics.percentile(0.0), 10.0);
        assert_eq!(metrics.percentile(0.5), 30.0);
        assert_eq!(metrics.percentile(1.0), 50.0);
    }

    #[test]
    fn test_avg_latency() {
        let mut metrics = LoadTestMetrics::new();
        
        metrics.record_success(Duration::from_millis(10));
        metrics.record_success(Duration::from_millis(20));
        metrics.record_success(Duration::from_millis(30));
        
        assert_eq!(metrics.avg_latency_ms(), 20.0);
    }

    #[test]
    fn test_max_latency() {
        let mut metrics = LoadTestMetrics::new();
        
        metrics.record_success(Duration::from_millis(10));
        metrics.record_success(Duration::from_millis(50));
        metrics.record_success(Duration::from_millis(30));
        
        assert_eq!(metrics.max_latency_ms(), 50.0);
    }
}
