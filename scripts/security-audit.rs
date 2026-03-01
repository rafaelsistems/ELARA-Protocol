#!/usr/bin/env rust-script
//! Security Audit Script for ELARA Protocol
//!
//! This script automates dependency vulnerability scanning using cargo-audit.
//! It provides configurable severity filtering, allow-list support, and
//! structured reporting for CI/CD integration.
//!
//! # Usage
//!
//! ```bash
//! # Run with default configuration (Medium severity threshold)
//! cargo run --manifest-path scripts/security-audit.rs
//!
//! # Run with custom severity threshold
//! AUDIT_SEVERITY=High cargo run --manifest-path scripts/security-audit.rs
//!
//! # Run with allow-list
//! AUDIT_ALLOW_LIST=RUSTSEC-2023-0001,RUSTSEC-2023-0002 cargo run --manifest-path scripts/security-audit.rs
//!
//! # Disable yanked crate checking
//! AUDIT_CHECK_YANKED=false cargo run --manifest-path scripts/security-audit.rs
//! ```
//!
//! # Exit Codes
//!
//! - 0: Audit passed (no vulnerabilities above threshold)
//! - 1: Audit failed (vulnerabilities found)
//! - 2: Configuration error
//! - 3: Execution error (cargo-audit not found, etc.)

use std::collections::HashSet;
use std::env;
use std::fmt;
use std::process::{Command, ExitCode};
use std::str::FromStr;

/// Severity levels for vulnerability classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Low => write!(f, "Low"),
            Severity::Medium => write!(f, "Medium"),
            Severity::High => write!(f, "High"),
            Severity::Critical => write!(f, "Critical"),
        }
    }
}

impl FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Severity::Low),
            "medium" => Ok(Severity::Medium),
            "high" => Ok(Severity::High),
            "critical" => Ok(Severity::Critical),
            _ => Err(format!("Invalid severity level: {}", s)),
        }
    }
}

/// Configuration for security audit execution
#[derive(Debug)]
struct AuditConfig {
    /// Fail on vulnerabilities with severity >= threshold
    severity_threshold: Severity,
    /// Allow-list for known acceptable vulnerabilities
    allowed_vulnerabilities: HashSet<String>,
    /// Check for yanked crates
    check_yanked: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            severity_threshold: Severity::Medium,
            allowed_vulnerabilities: HashSet::new(),
            check_yanked: true,
        }
    }
}

impl AuditConfig {
    /// Load configuration from environment variables
    fn from_env() -> Result<Self, String> {
        let mut config = Self::default();

        // Parse severity threshold
        if let Ok(severity_str) = env::var("AUDIT_SEVERITY") {
            config.severity_threshold = Severity::from_str(&severity_str)?;
        }

        // Parse allow-list
        if let Ok(allow_list) = env::var("AUDIT_ALLOW_LIST") {
            config.allowed_vulnerabilities = allow_list
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        // Parse yanked check flag
        if let Ok(check_yanked_str) = env::var("AUDIT_CHECK_YANKED") {
            config.check_yanked = check_yanked_str.to_lowercase() != "false";
        }

        Ok(config)
    }
}

/// Represents a single vulnerability finding
#[derive(Debug)]
struct Vulnerability {
    id: String,
    crate_name: String,
    version: String,
    severity: Severity,
    description: String,
    advisory_url: String,
    patched_versions: Vec<String>,
}

impl fmt::Display for Vulnerability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  [{}] {} v{}", self.severity, self.crate_name, self.version)?;
        writeln!(f, "    ID: {}", self.id)?;
        writeln!(f, "    Description: {}", self.description)?;
        writeln!(f, "    Advisory: {}", self.advisory_url)?;
        if !self.patched_versions.is_empty() {
            writeln!(f, "    Patched versions: {}", self.patched_versions.join(", "))?;
        }
        Ok(())
    }
}

/// Results of a security audit
#[derive(Debug)]
struct AuditReport {
    vulnerabilities: Vec<Vulnerability>,
    yanked_crates: Vec<String>,
    passed: bool,
}

impl AuditReport {
    fn new() -> Self {
        Self {
            vulnerabilities: Vec::new(),
            yanked_crates: Vec::new(),
            passed: true,
        }
    }
}

/// Main security audit orchestrator
struct SecurityAudit {
    config: AuditConfig,
}

impl SecurityAudit {
    fn new(config: AuditConfig) -> Self {
        Self { config }
    }

    /// Execute the security audit
    fn run(&self) -> Result<AuditReport, String> {
        println!("🔍 Running ELARA Protocol Security Audit");
        println!("   Severity threshold: {}", self.config.severity_threshold);
        println!("   Check yanked crates: {}", self.config.check_yanked);
        if !self.config.allowed_vulnerabilities.is_empty() {
            println!("   Allow-listed vulnerabilities: {}", 
                self.config.allowed_vulnerabilities.len());
        }
        println!();

        // Step 1: Check if cargo-audit is installed
        self.check_cargo_audit_installed()?;

        // Step 2: Update advisory database
        println!("📥 Updating advisory database...");
        self.update_advisory_database()?;

        // Step 3: Run cargo-audit
        println!("🔎 Scanning dependencies for vulnerabilities...");
        let audit_output = self.execute_cargo_audit()?;

        // Step 4: Parse vulnerabilities
        let mut report = self.parse_audit_output(&audit_output)?;

        // Step 5: Check for yanked crates if enabled
        if self.config.check_yanked {
            println!("📦 Checking for yanked crates...");
            report.yanked_crates = self.check_yanked_crates()?;
        }

        // Step 6: Determine pass/fail
        report.passed = report.vulnerabilities.is_empty() && report.yanked_crates.is_empty();

        Ok(report)
    }

    /// Check if cargo-audit is installed
    fn check_cargo_audit_installed(&self) -> Result<(), String> {
        let output = Command::new("cargo")
            .arg("audit")
            .arg("--version")
            .output()
            .map_err(|e| format!("Failed to check cargo-audit installation: {}", e))?;

        if !output.status.success() {
            return Err(
                "cargo-audit is not installed. Install it with: cargo install cargo-audit"
                    .to_string(),
            );
        }

        Ok(())
    }

    /// Update the advisory database
    fn update_advisory_database(&self) -> Result<(), String> {
        // Try to update the advisory database
        // Note: Some versions of cargo-audit don't have a 'fetch' subcommand
        // In those cases, the database is updated automatically during audit
        let output = Command::new("cargo")
            .arg("audit")
            .arg("fetch")
            .output();

        match output {
            Ok(out) if out.status.success() => {
                // Successfully updated
                Ok(())
            }
            Ok(out) => {
                // Check if it's just an unrecognized subcommand error
                let stderr = String::from_utf8_lossy(&out.stderr);
                if stderr.contains("unrecognized subcommand") {
                    // This version doesn't support fetch, but that's okay
                    // The database will be updated during the audit run
                    println!("   (Database will be updated during audit)");
                    Ok(())
                } else {
                    Err(format!("Failed to update advisory database: {}", stderr))
                }
            }
            Err(e) => Err(format!("Failed to execute cargo audit fetch: {}", e)),
        }
    }

    /// Execute cargo-audit and return raw output
    fn execute_cargo_audit(&self) -> Result<String, String> {
        let output = Command::new("cargo")
            .arg("audit")
            .arg("--json")
            .output()
            .map_err(|e| format!("Failed to execute cargo-audit: {}", e))?;

        // Note: cargo-audit returns non-zero exit code when vulnerabilities are found
        // This is expected behavior, so we don't check status here
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    }

    /// Parse cargo-audit JSON output
    fn parse_audit_output(&self, output: &str) -> Result<AuditReport, String> {
        let mut report = AuditReport::new();

        // Parse JSON output line by line (cargo-audit outputs JSONL format)
        for line in output.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Simple JSON parsing for vulnerability entries
            // In production, you'd use serde_json, but for a standalone script we keep it simple
            if line.contains("\"type\":\"warning\"") && line.contains("\"kind\":\"vulnerability\"") {
                if let Some(vuln) = self.parse_vulnerability_line(line) {
                    // Apply severity filter
                    if vuln.severity >= self.config.severity_threshold {
                        // Apply allow-list
                        if !self.config.allowed_vulnerabilities.contains(&vuln.id) {
                            report.vulnerabilities.push(vuln);
                        }
                    }
                }
            }
        }

        Ok(report)
    }

    /// Parse a single vulnerability from JSON line
    fn parse_vulnerability_line(&self, line: &str) -> Option<Vulnerability> {
        // Extract fields using simple string parsing
        // In production, use serde_json for robust parsing
        
        let id = self.extract_json_field(line, "id")?;
        let crate_name = self.extract_json_field(line, "package")?;
        let version = self.extract_json_field(line, "version")?;
        let description = self.extract_json_field(line, "title")
            .or_else(|| self.extract_json_field(line, "description"))?;
        let advisory_url = self.extract_json_field(line, "url")
            .unwrap_or_else(|| format!("https://rustsec.org/advisories/{}", id));

        // Parse severity (default to Medium if not found)
        let severity = self.extract_json_field(line, "severity")
            .and_then(|s| Severity::from_str(&s).ok())
            .unwrap_or(Severity::Medium);

        // Extract patched versions (simplified)
        let patched_versions = self.extract_json_array(line, "patched_versions")
            .unwrap_or_default();

        Some(Vulnerability {
            id,
            crate_name,
            version,
            severity,
            description,
            advisory_url,
            patched_versions,
        })
    }

    /// Extract a JSON string field value
    fn extract_json_field(&self, json: &str, field: &str) -> Option<String> {
        let pattern = format!("\"{}\":\"", field);
        let start = json.find(&pattern)? + pattern.len();
        let end = json[start..].find('"')? + start;
        Some(json[start..end].to_string())
    }

    /// Extract a JSON array field (simplified)
    fn extract_json_array(&self, json: &str, field: &str) -> Option<Vec<String>> {
        let pattern = format!("\"{}\":[", field);
        let start = json.find(&pattern)? + pattern.len();
        let end = json[start..].find(']')? + start;
        let array_content = &json[start..end];
        
        let items: Vec<String> = array_content
            .split(',')
            .filter_map(|s| {
                let trimmed = s.trim().trim_matches('"');
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
            .collect();
        
        Some(items)
    }

    /// Check for yanked crates
    fn check_yanked_crates(&self) -> Result<Vec<String>, String> {
        let output = Command::new("cargo")
            .arg("audit")
            .arg("--json")
            .arg("--deny")
            .arg("yanked")
            .output()
            .map_err(|e| format!("Failed to check yanked crates: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut yanked = Vec::new();

        for line in stdout.lines() {
            if line.contains("\"type\":\"warning\"") && line.contains("\"kind\":\"yanked\"") {
                if let Some(crate_name) = self.extract_json_field(line, "package") {
                    if let Some(version) = self.extract_json_field(line, "version") {
                        yanked.push(format!("{} v{}", crate_name, version));
                    }
                }
            }
        }

        Ok(yanked)
    }

    /// Print the audit report
    fn print_report(&self, report: &AuditReport) {
        println!();
        println!("═══════════════════════════════════════════════════════════");
        println!("                  SECURITY AUDIT REPORT");
        println!("═══════════════════════════════════════════════════════════");
        println!();

        if report.vulnerabilities.is_empty() && report.yanked_crates.is_empty() {
            println!("✅ No security issues found!");
            println!();
            println!("All dependencies are secure and up-to-date.");
        } else {
            if !report.vulnerabilities.is_empty() {
                println!("❌ VULNERABILITIES FOUND: {}", report.vulnerabilities.len());
                println!();
                for vuln in &report.vulnerabilities {
                    println!("{}", vuln);
                }
            }

            if !report.yanked_crates.is_empty() {
                println!("⚠️  YANKED CRATES FOUND: {}", report.yanked_crates.len());
                println!();
                for yanked in &report.yanked_crates {
                    println!("  - {}", yanked);
                }
                println!();
            }

            println!("═══════════════════════════════════════════════════════════");
            println!();
            println!("🔧 REMEDIATION STEPS:");
            println!();
            println!("1. Review each vulnerability and assess impact");
            println!("2. Update affected dependencies to patched versions");
            println!("3. Run 'cargo update' to update Cargo.lock");
            println!("4. Re-run this audit to verify fixes");
            println!();
            println!("For more information:");
            println!("  - RustSec Advisory Database: https://rustsec.org/");
            println!("  - cargo-audit documentation: https://github.com/rustsec/rustsec");
        }

        println!("═══════════════════════════════════════════════════════════");
        println!();
    }
}

fn main() -> ExitCode {
    // Load configuration from environment
    let config = match AuditConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("❌ Configuration error: {}", e);
            return ExitCode::from(2);
        }
    };

    // Create and run audit
    let audit = SecurityAudit::new(config);
    let report = match audit.run() {
        Ok(report) => report,
        Err(e) => {
            eprintln!("❌ Audit execution error: {}", e);
            return ExitCode::from(3);
        }
    };

    // Print report
    audit.print_report(&report);

    // Return appropriate exit code
    if report.passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Low < Severity::Medium);
        assert!(Severity::Medium < Severity::High);
        assert!(Severity::High < Severity::Critical);
    }

    #[test]
    fn test_severity_from_str() {
        assert_eq!(Severity::from_str("low").unwrap(), Severity::Low);
        assert_eq!(Severity::from_str("Medium").unwrap(), Severity::Medium);
        assert_eq!(Severity::from_str("HIGH").unwrap(), Severity::High);
        assert_eq!(Severity::from_str("critical").unwrap(), Severity::Critical);
        assert!(Severity::from_str("invalid").is_err());
    }

    #[test]
    fn test_default_config() {
        let config = AuditConfig::default();
        assert_eq!(config.severity_threshold, Severity::Medium);
        assert!(config.allowed_vulnerabilities.is_empty());
        assert!(config.check_yanked);
    }

    #[test]
    fn test_extract_json_field() {
        let audit = SecurityAudit::new(AuditConfig::default());
        let json = r#"{"id":"RUSTSEC-2023-0001","package":"test-crate","version":"1.0.0"}"#;
        
        assert_eq!(audit.extract_json_field(json, "id"), Some("RUSTSEC-2023-0001".to_string()));
        assert_eq!(audit.extract_json_field(json, "package"), Some("test-crate".to_string()));
        assert_eq!(audit.extract_json_field(json, "version"), Some("1.0.0".to_string()));
        assert_eq!(audit.extract_json_field(json, "nonexistent"), None);
    }
}
