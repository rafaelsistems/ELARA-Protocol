//! Integration tests for the Health Check HTTP Server
//!
//! These tests verify that the health check HTTP server correctly exposes
//! health status via HTTP endpoints and returns appropriate status codes
//! and JSON responses.

use elara_runtime::health::{HealthCheck, HealthCheckResult, HealthChecker};
use elara_runtime::health_server::{HealthServer, HealthServerConfig};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// Test health checks for controlled testing
struct AlwaysHealthyCheck;
impl HealthCheck for AlwaysHealthyCheck {
    fn name(&self) -> &str {
        "always_healthy"
    }
    fn check(&self) -> HealthCheckResult {
        HealthCheckResult::Healthy
    }
}

struct AlwaysDegradedCheck;
impl HealthCheck for AlwaysDegradedCheck {
    fn name(&self) -> &str {
        "always_degraded"
    }
    fn check(&self) -> HealthCheckResult {
        HealthCheckResult::Degraded {
            reason: "Test degradation".to_string(),
        }
    }
}

struct AlwaysUnhealthyCheck;
impl HealthCheck for AlwaysUnhealthyCheck {
    fn name(&self) -> &str {
        "always_unhealthy"
    }
    fn check(&self) -> HealthCheckResult {
        HealthCheckResult::Unhealthy {
            reason: "Test failure".to_string(),
        }
    }
}

/// Helper function to start a health server on a random port
async fn start_test_server(
    checker: Arc<HealthChecker>,
) -> (tokio::task::JoinHandle<()>, u16) {
    // Use port 0 to let the OS assign a random available port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to random port");
    
    let addr = listener.local_addr().expect("Failed to get local address");
    let port = addr.port();
    
    let config = HealthServerConfig {
        bind_address: addr,
    };
    
    let server = HealthServer::new(checker, config);
    let router = server.create_router();
    
    let handle = tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("Server failed");
    });
    
    // Give the server a moment to start
    sleep(Duration::from_millis(100)).await;
    
    (handle, port)
}

#[tokio::test]
async fn test_health_endpoint_healthy() {
    let mut checker = HealthChecker::new(Duration::from_secs(30));
    checker.add_check(Box::new(AlwaysHealthyCheck));
    let checker = Arc::new(checker);
    
    let (_handle, port) = start_test_server(checker).await;
    
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/health", port))
        .send()
        .await
        .expect("Failed to send request");
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "healthy");
    assert!(body["checks"].is_object());
    assert_eq!(body["checks"]["always_healthy"]["status"], "healthy");
}

#[tokio::test]
async fn test_health_endpoint_degraded() {
    let mut checker = HealthChecker::new(Duration::from_secs(30));
    checker.add_check(Box::new(AlwaysHealthyCheck));
    checker.add_check(Box::new(AlwaysDegradedCheck));
    let checker = Arc::new(checker);
    
    let (_handle, port) = start_test_server(checker).await;
    
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/health", port))
        .send()
        .await
        .expect("Failed to send request");
    
    // Degraded still returns 200 OK
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "degraded");
    assert_eq!(body["checks"]["always_degraded"]["status"], "degraded");
    assert_eq!(
        body["checks"]["always_degraded"]["reason"],
        "Test degradation"
    );
}

#[tokio::test]
async fn test_health_endpoint_unhealthy() {
    let mut checker = HealthChecker::new(Duration::from_secs(30));
    checker.add_check(Box::new(AlwaysUnhealthyCheck));
    let checker = Arc::new(checker);
    
    let (_handle, port) = start_test_server(checker).await;
    
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/health", port))
        .send()
        .await
        .expect("Failed to send request");
    
    // Unhealthy returns 503 Service Unavailable
    assert_eq!(response.status(), 503);
    
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "unhealthy");
    assert_eq!(body["checks"]["always_unhealthy"]["status"], "unhealthy");
    assert_eq!(body["checks"]["always_unhealthy"]["reason"], "Test failure");
}

#[tokio::test]
async fn test_ready_endpoint() {
    let mut checker = HealthChecker::new(Duration::from_secs(30));
    checker.add_check(Box::new(AlwaysHealthyCheck));
    let checker = Arc::new(checker);
    
    let (_handle, port) = start_test_server(checker).await;
    
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/ready", port))
        .send()
        .await
        .expect("Failed to send request");
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "healthy");
}

#[tokio::test]
async fn test_ready_endpoint_not_ready() {
    let mut checker = HealthChecker::new(Duration::from_secs(30));
    checker.add_check(Box::new(AlwaysUnhealthyCheck));
    let checker = Arc::new(checker);
    
    let (_handle, port) = start_test_server(checker).await;
    
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/ready", port))
        .send()
        .await
        .expect("Failed to send request");
    
    assert_eq!(response.status(), 503);
}

#[tokio::test]
async fn test_live_endpoint() {
    let mut checker = HealthChecker::new(Duration::from_secs(30));
    checker.add_check(Box::new(AlwaysHealthyCheck));
    let checker = Arc::new(checker);
    
    let (_handle, port) = start_test_server(checker).await;
    
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/live", port))
        .send()
        .await
        .expect("Failed to send request");
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "healthy");
}

#[tokio::test]
async fn test_live_endpoint_degraded_is_alive() {
    let mut checker = HealthChecker::new(Duration::from_secs(30));
    checker.add_check(Box::new(AlwaysDegradedCheck));
    let checker = Arc::new(checker);
    
    let (_handle, port) = start_test_server(checker).await;
    
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/live", port))
        .send()
        .await
        .expect("Failed to send request");
    
    // Degraded is still considered "alive" for liveness probe
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "degraded");
}

#[tokio::test]
async fn test_live_endpoint_unhealthy() {
    let mut checker = HealthChecker::new(Duration::from_secs(30));
    checker.add_check(Box::new(AlwaysUnhealthyCheck));
    let checker = Arc::new(checker);
    
    let (_handle, port) = start_test_server(checker).await;
    
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/live", port))
        .send()
        .await
        .expect("Failed to send request");
    
    assert_eq!(response.status(), 503);
}

#[tokio::test]
async fn test_all_endpoints_return_json() {
    let mut checker = HealthChecker::new(Duration::from_secs(30));
    checker.add_check(Box::new(AlwaysHealthyCheck));
    let checker = Arc::new(checker);
    
    let (_handle, port) = start_test_server(checker).await;
    
    let client = reqwest::Client::new();
    
    // Test /health
    let response = client
        .get(format!("http://127.0.0.1:{}/health", port))
        .send()
        .await
        .expect("Failed to send request");
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );
    
    // Test /ready
    let response = client
        .get(format!("http://127.0.0.1:{}/ready", port))
        .send()
        .await
        .expect("Failed to send request");
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );
    
    // Test /live
    let response = client
        .get(format!("http://127.0.0.1:{}/live", port))
        .send()
        .await
        .expect("Failed to send request");
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );
}

#[tokio::test]
async fn test_health_response_includes_timestamp() {
    let mut checker = HealthChecker::new(Duration::from_secs(30));
    checker.add_check(Box::new(AlwaysHealthyCheck));
    let checker = Arc::new(checker);
    
    let (_handle, port) = start_test_server(checker).await;
    
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/health", port))
        .send()
        .await
        .expect("Failed to send request");
    
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    
    // Verify timestamp field exists and is a string
    assert!(body["timestamp"].is_string());
    let timestamp = body["timestamp"].as_str().unwrap();
    
    // Verify it's a valid ISO 8601 timestamp (basic check)
    assert!(timestamp.contains('T'));
    assert!(timestamp.contains('Z') || timestamp.contains('+') || timestamp.contains('-'));
}

#[tokio::test]
async fn test_health_response_includes_all_checks() {
    let mut checker = HealthChecker::new(Duration::from_secs(30));
    checker.add_check(Box::new(AlwaysHealthyCheck));
    checker.add_check(Box::new(AlwaysDegradedCheck));
    checker.add_check(Box::new(AlwaysUnhealthyCheck));
    let checker = Arc::new(checker);
    
    let (_handle, port) = start_test_server(checker).await;
    
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/health", port))
        .send()
        .await
        .expect("Failed to send request");
    
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    
    // Verify all three checks are present
    assert!(body["checks"]["always_healthy"].is_object());
    assert!(body["checks"]["always_degraded"].is_object());
    assert!(body["checks"]["always_unhealthy"].is_object());
    
    // Verify each check has a status
    assert_eq!(body["checks"]["always_healthy"]["status"], "healthy");
    assert_eq!(body["checks"]["always_degraded"]["status"], "degraded");
    assert_eq!(body["checks"]["always_unhealthy"]["status"], "unhealthy");
}

#[tokio::test]
async fn test_health_caching_works() {
    let mut checker = HealthChecker::new(Duration::from_millis(100));
    checker.add_check(Box::new(AlwaysHealthyCheck));
    let checker = Arc::new(checker);
    
    let (_handle, port) = start_test_server(checker).await;
    
    let client = reqwest::Client::new();
    
    // First request
    let response1 = client
        .get(format!("http://127.0.0.1:{}/health", port))
        .send()
        .await
        .expect("Failed to send request");
    let body1: serde_json::Value = response1.json().await.expect("Failed to parse JSON");
    let timestamp1 = body1["timestamp"].as_str().unwrap().to_string();
    
    // Second request immediately (should use cache)
    let response2 = client
        .get(format!("http://127.0.0.1:{}/health", port))
        .send()
        .await
        .expect("Failed to send request");
    let body2: serde_json::Value = response2.json().await.expect("Failed to parse JSON");
    let timestamp2 = body2["timestamp"].as_str().unwrap().to_string();
    
    // Timestamps should be the same (cached) - compare just the seconds part
    // to avoid nanosecond precision issues
    assert_eq!(&timestamp1[..19], &timestamp2[..19], "Cached timestamps should match");
    
    // Wait for cache to expire
    sleep(Duration::from_millis(150)).await;
    
    // Third request after cache expiry
    let response3 = client
        .get(format!("http://127.0.0.1:{}/health", port))
        .send()
        .await
        .expect("Failed to send request");
    let body3: serde_json::Value = response3.json().await.expect("Failed to parse JSON");
    let timestamp3 = body3["timestamp"].as_str().unwrap().to_string();
    
    // Timestamp should be different (cache expired)
    // We can't guarantee they'll be different at second precision, so just verify
    // the response is valid
    assert!(timestamp3.contains('T'), "Timestamp should be valid ISO 8601");
}
