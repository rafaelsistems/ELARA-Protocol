# ELARA Protocol Security Automation Scripts

This directory contains security automation scripts for the ELARA Protocol project.

## Scripts

### security-audit.rs

Automated dependency vulnerability scanning using `cargo-audit`.

### generate-sbom.rs

Software Bill of Materials (SBOM) generation in CycloneDX format.

## security-audit.rs

Automated dependency vulnerability scanning using `cargo-audit`.

### Features

- **Configurable Severity Filtering**: Set minimum severity threshold (Low, Medium, High, Critical)
- **Allow-list Support**: Exclude known acceptable vulnerabilities
- **Yanked Crate Detection**: Identify dependencies that have been yanked from crates.io
- **Structured Reporting**: Clear, actionable security reports
- **CI/CD Integration**: Exit codes for automated pipeline integration
- **Zero External Dependencies**: Uses only Rust standard library for maximum portability

### Prerequisites

Install `cargo-audit`:

```bash
cargo install cargo-audit
```

### Usage

#### Basic Usage

Run with default configuration (Medium severity threshold):

```bash
cargo run --manifest-path scripts/Cargo.toml
```

Or directly:

```bash
cargo run --bin security-audit --manifest-path scripts/Cargo.toml
```

#### Configuration via Environment Variables

**Set Severity Threshold:**

```bash
# Only fail on High or Critical vulnerabilities
AUDIT_SEVERITY=High cargo run --manifest-path scripts/Cargo.toml

# Fail on any vulnerability (including Low)
AUDIT_SEVERITY=Low cargo run --manifest-path scripts/Cargo.toml
```

**Allow-list Vulnerabilities:**

```bash
# Exclude specific vulnerabilities from failing the audit
AUDIT_ALLOW_LIST=RUSTSEC-2023-0001,RUSTSEC-2023-0002 cargo run --manifest-path scripts/Cargo.toml
```

**Disable Yanked Crate Checking:**

```bash
# Skip checking for yanked crates
AUDIT_CHECK_YANKED=false cargo run --manifest-path scripts/Cargo.toml
```

**Combined Configuration:**

```bash
AUDIT_SEVERITY=High \
AUDIT_ALLOW_LIST=RUSTSEC-2023-0001 \
AUDIT_CHECK_YANKED=true \
cargo run --manifest-path scripts/Cargo.toml
```

### Exit Codes

- **0**: Audit passed (no vulnerabilities above threshold)
- **1**: Audit failed (vulnerabilities found)
- **2**: Configuration error (invalid severity level, etc.)
- **3**: Execution error (cargo-audit not installed, etc.)

### CI/CD Integration

#### GitHub Actions Example

```yaml
name: Security Audit

on:
  schedule:
    - cron: '0 0 * * *'  # Daily at midnight
  push:
    branches: [main]
  pull_request:

jobs:
  security-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: Install cargo-audit
        run: cargo install cargo-audit
        
      - name: Run Security Audit
        env:
          AUDIT_SEVERITY: Medium
          AUDIT_CHECK_YANKED: true
        run: cargo run --manifest-path scripts/Cargo.toml
```

### Output Examples

#### Successful Audit

```
🔍 Running ELARA Protocol Security Audit
   Severity threshold: Medium
   Check yanked crates: true

📥 Updating advisory database...
🔎 Scanning dependencies for vulnerabilities...
📦 Checking for yanked crates...

═══════════════════════════════════════════════════════════
                  SECURITY AUDIT REPORT
═══════════════════════════════════════════════════════════

✅ No security issues found!

All dependencies are secure and up-to-date.
═══════════════════════════════════════════════════════════
```

#### Failed Audit

```
🔍 Running ELARA Protocol Security Audit
   Severity threshold: Medium
   Check yanked crates: true

📥 Updating advisory database...
🔎 Scanning dependencies for vulnerabilities...
📦 Checking for yanked crates...

═══════════════════════════════════════════════════════════
                  SECURITY AUDIT REPORT
═══════════════════════════════════════════════════════════

❌ VULNERABILITIES FOUND: 2

  [High] tokio v1.28.0
    ID: RUSTSEC-2023-0001
    Description: Data race in tokio runtime
    Advisory: https://rustsec.org/advisories/RUSTSEC-2023-0001
    Patched versions: >=1.28.1

  [Critical] openssl v0.10.45
    ID: RUSTSEC-2023-0002
    Description: Memory corruption in OpenSSL
    Advisory: https://rustsec.org/advisories/RUSTSEC-2023-0002
    Patched versions: >=0.10.46

═══════════════════════════════════════════════════════════

🔧 REMEDIATION STEPS:

1. Review each vulnerability and assess impact
2. Update affected dependencies to patched versions
3. Run 'cargo update' to update Cargo.lock
4. Re-run this audit to verify fixes

For more information:
  - RustSec Advisory Database: https://rustsec.org/
  - cargo-audit documentation: https://github.com/rustsec/rustsec
═══════════════════════════════════════════════════════════
```

### Configuration Reference

| Environment Variable | Values | Default | Description |
|---------------------|--------|---------|-------------|
| `AUDIT_SEVERITY` | `Low`, `Medium`, `High`, `Critical` | `Medium` | Minimum severity level to fail on |
| `AUDIT_ALLOW_LIST` | Comma-separated RUSTSEC IDs | (empty) | Vulnerabilities to exclude from failures |
| `AUDIT_CHECK_YANKED` | `true`, `false` | `true` | Whether to check for yanked crates |

### Development

#### Running Tests

```bash
cargo test --manifest-path scripts/Cargo.toml
```

#### Building Release Version

```bash
cargo build --release --manifest-path scripts/Cargo.toml
```

The compiled binary will be at `scripts/target/release/security-audit` (or `.exe` on Windows).

### Implementation Details

The script implements the `SecurityAudit` struct with the following key components:

- **AuditConfig**: Configuration for severity thresholds, allow-lists, and yanked crate checking
- **Vulnerability**: Structured representation of security vulnerabilities
- **AuditReport**: Aggregated results with pass/fail status
- **JSON Parsing**: Simple JSON parsing for cargo-audit output (no external dependencies)

The implementation follows the design specified in `.kiro/specs/production-readiness-implementation/design.md`.

### Troubleshooting

**Error: cargo-audit is not installed**

Install cargo-audit:
```bash
cargo install cargo-audit
```

**Error: Failed to update advisory database**

This usually indicates network connectivity issues. The script will still attempt to run the audit with the cached database.

**Error: Invalid severity level**

Ensure `AUDIT_SEVERITY` is one of: `Low`, `Medium`, `High`, `Critical` (case-insensitive).

### Related Documentation

- [RustSec Advisory Database](https://rustsec.org/)
- [cargo-audit Documentation](https://github.com/rustsec/rustsec/tree/main/cargo-audit)
- [ELARA Protocol Security Policy](../SECURITY.md)
- [Production Readiness Spec](.kiro/specs/production-readiness-implementation/)


---

## generate-sbom.rs

Software Bill of Materials (SBOM) generation in CycloneDX format.

### Features

- **CycloneDX 1.5 Format**: Industry-standard SBOM format
- **Complete Dependency Scanning**: Scans all workspace dependencies
- **License Information**: Includes license data for each component
- **Dependency Tree**: Captures dependency relationships
- **Package URLs (purl)**: Standard package identifiers
- **Checksum Support**: Includes SHA-256 checksums when available
- **Zero External Dependencies**: Uses only Rust standard library

### Prerequisites

No additional tools required - uses built-in `cargo metadata` command.

### Usage

#### Basic Usage

Generate SBOM with default output (sbom.json):

```bash
cargo run --bin generate-sbom --manifest-path scripts/Cargo.toml
```

#### Configuration via Environment Variables

**Custom Output Path:**

```bash
# Specify custom output file
SBOM_OUTPUT=./artifacts/sbom.json cargo run --bin generate-sbom --manifest-path scripts/Cargo.toml

# Output to release directory
SBOM_OUTPUT=./target/release/sbom.json cargo run --bin generate-sbom --manifest-path scripts/Cargo.toml
```

**Generate for Specific Package:**

```bash
# Generate SBOM for a specific workspace member
SBOM_PACKAGE=elara-core cargo run --bin generate-sbom --manifest-path scripts/Cargo.toml
```

**Include Dev Dependencies:**

```bash
# Include development dependencies in SBOM
SBOM_INCLUDE_DEV=true cargo run --bin generate-sbom --manifest-path scripts/Cargo.toml
```

**Combined Configuration:**

```bash
SBOM_OUTPUT=./release/sbom.json \
SBOM_PACKAGE=elara-runtime \
cargo run --bin generate-sbom --manifest-path scripts/Cargo.toml
```

### Exit Codes

- **0**: SBOM generated successfully
- **1**: Generation failed
- **2**: Configuration error
- **3**: Execution error

### Output Format

The script generates a CycloneDX 1.5 JSON SBOM with the following structure:

```json
{
  "bomFormat": "CycloneDX",
  "specVersion": "1.5",
  "version": 1,
  "metadata": {
    "timestamp": "2025-01-01T00:00:00Z",
    "tools": [...],
    "component": {
      "type": "application",
      "name": "elara-protocol",
      "version": "1.0.0"
    }
  },
  "components": [
    {
      "type": "library",
      "name": "tokio",
      "version": "1.35.0",
      "purl": "pkg:cargo/tokio@1.35.0",
      "licenses": [...],
      "hashes": [...],
      "bom-ref": "tokio@1.35.0"
    }
  ],
  "dependencies": [
    {
      "ref": "tokio@1.35.0",
      "dependsOn": ["bytes@1.5.0", "pin-project-lite@0.2.13"]
    }
  ]
}
```

### CI/CD Integration

#### GitHub Actions Example

```yaml
name: Generate SBOM

on:
  release:
    types: [published]
  push:
    branches: [main]

jobs:
  sbom:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: Generate SBOM
        env:
          SBOM_OUTPUT: ./artifacts/sbom.json
        run: |
          mkdir -p artifacts
          cargo run --bin generate-sbom --manifest-path scripts/Cargo.toml
          
      - name: Upload SBOM Artifact
        uses: actions/upload-artifact@v4
        with:
          name: sbom
          path: artifacts/sbom.json
          
      - name: Attach SBOM to Release
        if: github.event_name == 'release'
        uses: actions/upload-release-asset@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          upload_url: ${{ github.event.release.upload_url }}
          asset_path: ./artifacts/sbom.json
          asset_name: sbom.json
          asset_content_type: application/json
```

### Use Cases

**Supply Chain Security:**
- Track all dependencies and their versions
- Identify vulnerable components
- Verify software composition

**Compliance:**
- License compliance auditing
- Software composition analysis
- Regulatory requirements (e.g., SBOM requirements)

**Release Management:**
- Attach SBOM to releases
- Track dependency changes between versions
- Document software composition

**Security Analysis:**
- Input for vulnerability scanning tools
- Dependency risk assessment
- Software supply chain analysis

### SBOM Validation

Validate the generated SBOM using CycloneDX tools:

```bash
# Install cyclonedx-cli (optional)
npm install -g @cyclonedx/cyclonedx-cli

# Validate SBOM
cyclonedx-cli validate --input-file sbom.json

# Convert to other formats
cyclonedx-cli convert --input-file sbom.json --output-file sbom.xml --output-format xml
```

### Integration with Security Tools

The generated SBOM can be used with various security tools:

**Dependency-Track:**
```bash
# Upload SBOM to Dependency-Track for continuous monitoring
curl -X "POST" "https://dependency-track.example.com/api/v1/bom" \
  -H "X-Api-Key: $API_KEY" \
  -H "Content-Type: multipart/form-data" \
  -F "project=$PROJECT_UUID" \
  -F "bom=@sbom.json"
```

**Grype (Vulnerability Scanner):**
```bash
# Scan SBOM for vulnerabilities
grype sbom:sbom.json
```

**Syft (SBOM Analysis):**
```bash
# Analyze SBOM
syft sbom.json
```

### Output Examples

#### Successful Generation

```
📦 Generating SBOM for ELARA Protocol
   Output: sbom.json
   Scope: Entire workspace

🔍 Analyzing workspace dependencies...
   Found 1093 components
📝 Generating CycloneDX SBOM...
💾 Writing SBOM to sbom.json...

✅ SBOM generated successfully!
   File: sbom.json
   Format: CycloneDX 1.5 JSON
   Components: 1093
```

#### Package-Specific Generation

```
📦 Generating SBOM for ELARA Protocol
   Output: sbom.json
   Package: elara-core

🔍 Analyzing workspace dependencies...
   Found 42 components
📝 Generating CycloneDX SBOM...
💾 Writing SBOM to sbom.json...

✅ SBOM generated successfully!
   File: sbom.json
   Format: CycloneDX 1.5 JSON
   Components: 42
```

### Configuration Reference

| Environment Variable | Values | Default | Description |
|---------------------|--------|---------|-------------|
| `SBOM_OUTPUT` | File path | `sbom.json` | Output file path for the SBOM |
| `SBOM_PACKAGE` | Package name | (none) | Specific workspace package to generate SBOM for |
| `SBOM_INCLUDE_DEV` | `true`, `false` | `false` | Include development dependencies |

### Development

#### Running Tests

```bash
cargo test --bin generate-sbom --manifest-path scripts/Cargo.toml
```

#### Building Release Version

```bash
cargo build --release --bin generate-sbom --manifest-path scripts/Cargo.toml
```

The compiled binary will be at `scripts/target/release/generate-sbom` (or `.exe` on Windows).

### Implementation Details

The script implements the `SBOMGenerator` struct with the following key components:

- **SBOMConfig**: Configuration for output path, package selection, and dev dependencies
- **Component**: Structured representation of a software component with metadata
- **CycloneDX Generation**: Generates valid CycloneDX 1.5 JSON format
- **Dependency Tree**: Captures and represents dependency relationships
- **Metadata Extraction**: Extracts license, version, and checksum information

The implementation follows the design specified in `.kiro/specs/production-readiness-implementation/design.md`.

### Troubleshooting

**Error: cargo metadata failed**

Ensure you're running the script from the workspace root or a valid Cargo project directory.

**Error: Failed to write SBOM file**

Check that you have write permissions for the output directory. The script will create parent directories if they don't exist.

**Warning: Missing license information**

Some dependencies may not specify license information in their Cargo.toml. This is expected and the SBOM will be generated without license data for those components.

### Related Documentation

- [CycloneDX Specification](https://cyclonedx.org/specification/overview/)
- [NTIA Minimum Elements for SBOM](https://www.ntia.gov/report/2021/minimum-elements-software-bill-materials-sbom)
- [ELARA Protocol Security Policy](../SECURITY.md)
- [Production Readiness Spec](.kiro/specs/production-readiness-implementation/)

### Comparison with Other Tools

**vs. cargo-cyclonedx:**
- No external dependencies required
- Integrated into ELARA workflow
- Customizable output format
- Simpler configuration

**vs. syft:**
- Native Rust implementation
- Workspace-aware
- Direct cargo metadata integration
- Faster for Rust projects

**vs. cargo-sbom:**
- More control over output format
- Better workspace support
- Integrated with ELARA security automation
- Extensible for custom metadata
