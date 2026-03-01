//! Small deployment load test example
//!
//! This example runs a load test with 10 nodes, suitable for development
//! and initial performance validation.

use elara_loadtest::{LoadTestScenario, scenarios};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("ELARA Protocol Load Test - Small Deployment");
    println!("============================================\n");
    
    let config = scenarios::small_deployment();
    println!("Configuration:");
    println!("  Nodes: {}", config.num_nodes);
    println!("  Connections per node: {}", config.num_connections_per_node);
    println!("  Message rate: {} msg/sec", config.message_rate_per_second);
    println!("  Test duration: {:?}", config.test_duration);
    println!("  Ramp-up duration: {:?}\n", config.ramp_up_duration);
    
    let mut scenario = LoadTestScenario::new(config);
    
    println!("Starting load test...\n");
    let result = scenario.run().await?;
    
    println!("\n{}", result.report());
    
    // Print detailed errors if any
    if !result.errors.is_empty() {
        println!("Detailed Errors:");
        for (i, error) in result.errors.iter().enumerate().take(10) {
            println!("  {}. {}", i + 1, error);
        }
        if result.errors.len() > 10 {
            println!("  ... and {} more errors", result.errors.len() - 10);
        }
    }
    
    // Exit with error code if test had significant failures
    let failure_rate = result.failed_messages as f64 / result.total_messages as f64;
    if failure_rate > 0.05 {
        eprintln!("\nWARNING: Failure rate ({:.2}%) exceeds 5% threshold", failure_rate * 100.0);
        std::process::exit(1);
    }
    
    Ok(())
}
