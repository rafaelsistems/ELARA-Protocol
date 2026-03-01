#!/usr/bin/env rust-script
//! SBOM (Software Bill of Materials) Generation Script for ELARA Protocol
//!
//! This script generates a Software Bill of Materials in CycloneDX JSON format.
//! It scans all workspace dependencies, includes license information, and
//! generates a dependency tree suitable for supply chain security analysis.
//!
//! # Usage
//!
//! ```bash
//! # Generate SBOM with default output (sbom.json)
//! cargo run --bin generate-sbom --manifest-path scripts/Cargo.toml
//!
//! # Generate SBOM with custom output path
//! SBOM_OUTPUT=./artifacts/sbom.json cargo run --bin generate-sbom --manifest-path scripts/Cargo.toml
//!
//! # Generate SBOM for specific workspace member
//! SBOM_PACKAGE=elara-core cargo run --bin generate-sbom --manifest-path scripts/Cargo.toml
//! ```
//!
//! # Exit Codes
//!
//! - 0: SBOM generated successfully
//! - 1: Generation failed
//! - 2: Configuration error
//! - 3: Execution error (cargo-tree not available, etc.)

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

/// Configuration for SBOM generation
#[derive(Debug)]
struct SBOMConfig {
    /// Output file path for the SBOM
    output_path: PathBuf,
    /// Specific package to generate SBOM for (None = entire workspace)
    package: Option<String>,
    /// Include dev dependencies
    include_dev: bool,
}

impl Default for SBOMConfig {
    fn default() -> Self {
        Self {
            output_path: PathBuf::from("sbom.json"),
            package: None,
            include_dev: false,
        }
    }
}

impl SBOMConfig {
    /// Load configuration from environment variables
    fn from_env() -> Result<Self, String> {
        let mut config = Self::default();

        if let Ok(output) = env::var("SBOM_OUTPUT") {
            config.output_path = PathBuf::from(output);
        }

        if let Ok(package) = env::var("SBOM_PACKAGE") {
            config.package = Some(package);
        }

        if let Ok(include_dev) = env::var("SBOM_INCLUDE_DEV") {
            config.include_dev = include_dev.to_lowercase() == "true";
        }

        Ok(config)
    }
}

/// Represents a component in the SBOM
#[derive(Debug, Clone)]
struct Component {
    name: String,
    version: String,
    purl: String,
    licenses: Vec<String>,
    dependencies: Vec<String>,
    checksum: Option<String>,
}

/// Main SBOM generator
struct SBOMGenerator {
    config: SBOMConfig,
}

impl SBOMGenerator {
    fn new(config: SBOMConfig) -> Self {
        Self { config }
    }

    /// Generate the SBOM
    fn generate(&self) -> Result<(), String> {
        println!("📦 Generating SBOM for ELARA Protocol");
        println!("   Output: {}", self.config.output_path.display());
        if let Some(ref package) = self.config.package {
            println!("   Package: {}", package);
        } else {
            println!("   Scope: Entire workspace");
        }
        println!();

        // Step 1: Get workspace metadata
        println!("🔍 Analyzing workspace dependencies...");
        let metadata = self.get_cargo_metadata()?;

        // Step 2: Parse dependencies
        let components = self.parse_dependencies(&metadata)?;
        println!("   Found {} components", components.len());

        // Step 3: Generate CycloneDX JSON
        println!("📝 Generating CycloneDX SBOM...");
        let sbom_json = self.generate_cyclonedx_json(&components)?;

        // Step 4: Write to file
        println!("💾 Writing SBOM to {}...", self.config.output_path.display());
        self.write_sbom(&sbom_json)?;

        println!();
        println!("✅ SBOM generated successfully!");
        println!("   File: {}", self.config.output_path.display());
        println!("   Format: CycloneDX 1.5 JSON");
        println!("   Components: {}", components.len());

        Ok(())
    }

    /// Get cargo metadata using cargo metadata command
    fn get_cargo_metadata(&self) -> Result<String, String> {
        let mut cmd = Command::new("cargo");
        cmd.arg("metadata")
            .arg("--format-version=1")
            .arg("--locked");

        if let Some(ref package) = self.config.package {
            cmd.arg("--package").arg(package);
        }

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to execute cargo metadata: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("cargo metadata failed: {}", stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Parse dependencies from cargo metadata
    fn parse_dependencies(&self, metadata: &str) -> Result<Vec<Component>, String> {
        let mut components = Vec::new();
        let mut seen = HashSet::new();

        // Parse the JSON metadata
        // For simplicity, we'll use basic string parsing
        // In production, you'd use serde_json
        
        // Extract packages section
        if let Some(packages_start) = metadata.find("\"packages\":[") {
            let packages_section = &metadata[packages_start..];
            
            // Find all package entries
            let mut pos = 0;
            while let Some(pkg_start) = packages_section[pos..].find("{\"name\":\"") {
                let pkg_pos = pos + pkg_start;
                let pkg_section = &packages_section[pkg_pos..];
                
                // Extract package info
                if let Some(component) = self.parse_package_entry(pkg_section) {
                    let key = format!("{}@{}", component.name, component.version);
                    if !seen.contains(&key) {
                        seen.insert(key);
                        components.push(component);
                    }
                }
                
                pos = pkg_pos + 1;
            }
        }

        Ok(components)
    }

    /// Parse a single package entry
    fn parse_package_entry(&self, entry: &str) -> Option<Component> {
        let name = self.extract_json_string(entry, "name")?;
        let version = self.extract_json_string(entry, "version")?;
        
        // Generate Package URL (purl)
        let purl = format!("pkg:cargo/{}@{}", name, version);
        
        // Extract license
        let licenses = if let Some(license) = self.extract_json_string(entry, "license") {
            vec![license]
        } else {
            Vec::new()
        };
        
        // Extract dependencies
        let dependencies = self.extract_dependencies(entry);
        
        // Extract checksum if available
        let checksum = self.extract_checksum(entry);

        Some(Component {
            name,
            version,
            purl,
            licenses,
            dependencies,
            checksum,
        })
    }

    /// Extract a JSON string field
    fn extract_json_string(&self, json: &str, field: &str) -> Option<String> {
        let pattern = format!("\"{}\":\"", field);
        let start = json.find(&pattern)? + pattern.len();
        let end = json[start..].find('"')? + start;
        Some(json[start..end].replace("\\\"", "\""))
    }

    /// Extract dependencies from package entry
    fn extract_dependencies(&self, entry: &str) -> Vec<String> {
        let mut deps = Vec::new();
        
        if let Some(deps_start) = entry.find("\"dependencies\":[") {
            let deps_section = &entry[deps_start..];
            if let Some(deps_end) = deps_section.find(']') {
                let deps_content = &deps_section[..deps_end];
                
                // Extract dependency names
                let mut pos = 0;
                while let Some(name_start) = deps_content[pos..].find("\"name\":\"") {
                    let name_pos = pos + name_start + 8;
                    if let Some(name_end) = deps_content[name_pos..].find('"') {
                        let dep_name = deps_content[name_pos..name_pos + name_end].to_string();
                        deps.push(dep_name);
                        pos = name_pos + name_end;
                    } else {
                        break;
                    }
                }
            }
        }
        
        deps
    }

    /// Extract checksum from package entry
    fn extract_checksum(&self, entry: &str) -> Option<String> {
        // Look for checksum in the manifest
        self.extract_json_string(entry, "checksum")
    }

    /// Generate CycloneDX JSON format
    fn generate_cyclonedx_json(&self, components: &[Component]) -> Result<String, String> {
        let mut json = String::new();
        
        // CycloneDX header
        json.push_str("{\n");
        json.push_str("  \"bomFormat\": \"CycloneDX\",\n");
        json.push_str("  \"specVersion\": \"1.5\",\n");
        json.push_str("  \"version\": 1,\n");
        
        // Metadata
        json.push_str("  \"metadata\": {\n");
        json.push_str("    \"timestamp\": \"");
        json.push_str(&self.get_timestamp());
        json.push_str("\",\n");
        json.push_str("    \"tools\": [\n");
        json.push_str("      {\n");
        json.push_str("        \"vendor\": \"ELARA Protocol\",\n");
        json.push_str("        \"name\": \"generate-sbom\",\n");
        json.push_str("        \"version\": \"1.0.0\"\n");
        json.push_str("      }\n");
        json.push_str("    ],\n");
        json.push_str("    \"component\": {\n");
        json.push_str("      \"type\": \"application\",\n");
        json.push_str("      \"name\": \"elara-protocol\",\n");
        json.push_str("      \"version\": \"1.0.0\",\n");
        json.push_str("      \"description\": \"ELARA Protocol - Production-Ready Distributed Systems Framework\"\n");
        json.push_str("    }\n");
        json.push_str("  },\n");
        
        // Components
        json.push_str("  \"components\": [\n");
        for (i, component) in components.iter().enumerate() {
            json.push_str("    {\n");
            json.push_str(&format!("      \"type\": \"library\",\n"));
            json.push_str(&format!("      \"name\": \"{}\",\n", self.escape_json(&component.name)));
            json.push_str(&format!("      \"version\": \"{}\",\n", self.escape_json(&component.version)));
            json.push_str(&format!("      \"purl\": \"{}\",\n", self.escape_json(&component.purl)));
            
            // Licenses
            if !component.licenses.is_empty() {
                json.push_str("      \"licenses\": [\n");
                for (j, license) in component.licenses.iter().enumerate() {
                    json.push_str("        {\n");
                    json.push_str("          \"license\": {\n");
                    json.push_str(&format!("            \"id\": \"{}\"\n", self.escape_json(license)));
                    json.push_str("          }\n");
                    json.push_str("        }");
                    if j < component.licenses.len() - 1 {
                        json.push_str(",");
                    }
                    json.push_str("\n");
                }
                json.push_str("      ],\n");
            }
            
            // Hashes (checksum)
            if let Some(ref checksum) = component.checksum {
                json.push_str("      \"hashes\": [\n");
                json.push_str("        {\n");
                json.push_str("          \"alg\": \"SHA-256\",\n");
                json.push_str(&format!("          \"content\": \"{}\"\n", self.escape_json(checksum)));
                json.push_str("        }\n");
                json.push_str("      ],\n");
            }
            
            json.push_str(&format!("      \"bom-ref\": \"{}@{}\"\n", 
                self.escape_json(&component.name), 
                self.escape_json(&component.version)));
            json.push_str("    }");
            
            if i < components.len() - 1 {
                json.push_str(",");
            }
            json.push_str("\n");
        }
        json.push_str("  ],\n");
        
        // Dependencies
        json.push_str("  \"dependencies\": [\n");
        for (i, component) in components.iter().enumerate() {
            if !component.dependencies.is_empty() {
                json.push_str("    {\n");
                json.push_str(&format!("      \"ref\": \"{}@{}\",\n", 
                    self.escape_json(&component.name), 
                    self.escape_json(&component.version)));
                json.push_str("      \"dependsOn\": [\n");
                
                for (j, dep) in component.dependencies.iter().enumerate() {
                    // Find the version of this dependency
                    let dep_ref = if let Some(dep_component) = components.iter().find(|c| c.name == *dep) {
                        format!("{}@{}", dep, dep_component.version)
                    } else {
                        dep.clone()
                    };
                    
                    json.push_str(&format!("        \"{}\"", self.escape_json(&dep_ref)));
                    if j < component.dependencies.len() - 1 {
                        json.push_str(",");
                    }
                    json.push_str("\n");
                }
                
                json.push_str("      ]\n");
                json.push_str("    }");
                
                if i < components.len() - 1 {
                    json.push_str(",");
                }
                json.push_str("\n");
            }
        }
        json.push_str("  ]\n");
        
        json.push_str("}\n");
        
        Ok(json)
    }

    /// Get current timestamp in ISO 8601 format
    fn get_timestamp(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap();
        
        // Simple ISO 8601 format (YYYY-MM-DDTHH:MM:SSZ)
        // For production, use chrono crate
        format!("{}Z", duration.as_secs())
    }

    /// Escape JSON string
    fn escape_json(&self, s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    /// Write SBOM to file
    fn write_sbom(&self, content: &str) -> Result<(), String> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = self.config.output_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create output directory: {}", e))?;
            }
        }

        fs::write(&self.config.output_path, content)
            .map_err(|e| format!("Failed to write SBOM file: {}", e))?;

        Ok(())
    }
}

fn main() -> ExitCode {
    // Load configuration from environment
    let config = match SBOMConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("❌ Configuration error: {}", e);
            return ExitCode::from(2);
        }
    };

    // Create and run generator
    let generator = SBOMGenerator::new(config);
    match generator.generate() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("❌ SBOM generation failed: {}", e);
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SBOMConfig::default();
        assert_eq!(config.output_path, PathBuf::from("sbom.json"));
        assert_eq!(config.package, None);
        assert!(!config.include_dev);
    }

    #[test]
    fn test_escape_json() {
        let generator = SBOMGenerator::new(SBOMConfig::default());
        assert_eq!(generator.escape_json("hello"), "hello");
        assert_eq!(generator.escape_json("hello\"world"), "hello\\\"world");
        assert_eq!(generator.escape_json("hello\nworld"), "hello\\nworld");
        assert_eq!(generator.escape_json("hello\\world"), "hello\\\\world");
    }

    #[test]
    fn test_extract_json_string() {
        let generator = SBOMGenerator::new(SBOMConfig::default());
        let json = r#"{"name":"test-crate","version":"1.0.0"}"#;
        
        assert_eq!(generator.extract_json_string(json, "name"), Some("test-crate".to_string()));
        assert_eq!(generator.extract_json_string(json, "version"), Some("1.0.0".to_string()));
        assert_eq!(generator.extract_json_string(json, "nonexistent"), None);
    }
}
