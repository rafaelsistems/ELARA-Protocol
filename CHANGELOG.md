# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2024-01-15

### Changed

#### Version Management
- **Standardized workspace version management**: All 16 crates now use `version.workspace = true`
- **Centralized version control**: Single source of truth in workspace `Cargo.toml`
- **Removed hardcoded version constraints**: Internal dependencies now use path-only references

#### Updated Crates
All crates updated to version 0.2.0:
1. `elara-core` - Core types and primitives
2. `elara-wire` - Wire protocol encoding/decoding
3. `elara-crypto` - Cryptographic binding
4. `elara-time` - Time engine with dual clocks
5. `elara-state` - State reconciliation engine
6. `elara-transport` - Network transport abstraction
7. `elara-runtime` - Node runtime with observability
8. `elara-msp` - MSP profiles (Text, Voice)
9. `elara-test` - Testing harness and security tests
10. `elara-ffi` - Foreign function interface for mobile
11. `elara-visual` - Visual state encoding
12. `elara-diffusion` - Swarm diffusion protocols
13. `elara-voice` - Voice encoding and synthesis
14. `elara-fuzz` - Fuzzing infrastructure
15. `elara-bench` - Performance benchmarking suite
16. `elara-loadtest` - Load testing framework

#### Production Readiness Features
- **Observability Infrastructure**: Unified logging, metrics, and distributed tracing
- **Security Hardening**: Continuous fuzzing, dependency auditing, SBOM generation
- **Performance Validation**: Comprehensive benchmark suite and load testing framework
- **Operational Tooling**: Health checks, alerting, monitoring dashboards

### Improved
- **Dependency Management**: Cleaner dependency graph with workspace-level dependency management
- **Build Consistency**: Guaranteed version consistency across all crates
- **Maintenance**: Simplified version updates for future releases

### Fixed
- Inconsistent version management across crates
- Hardcoded version constraints in internal dependencies

## [0.1.0] - 2024-01-10

### Added
- Initial project structure with Rust workspace
- Core crates implementation:
  - `elara-core` - Core types and primitives
  - `elara-wire` - Wire protocol encoding/decoding
  - `elara-crypto` - Cryptographic binding (AEAD, ratchet, replay protection)
  - `elara-time` - Time engine with dual clocks
  - `elara-state` - State reconciliation engine
  - `elara-transport` - Network transport abstraction
  - `elara-runtime` - Node runtime
  - `elara-msp` - MSP v0 profiles (Textual, VoiceMinimal)
  - `elara-test` - Testing harness (TimeSimulator, StateFuzzer)
- Comprehensive documentation:
  - Architecture docs (core concepts, four pillars, representation profiles)
  - Specification docs (wire protocol, crypto binding, time engine, state reconciliation)
  - Implementation docs (crate structure, API reference, testing strategy)
  - MSP v0 specification
- Examples:
  - `basic_node` - Basic node creation and frame encryption
  - `time_simulation` - Time engine demonstration
  - `state_reconciliation` - State reconciliation concepts
- GitHub repository setup:
  - CI/CD workflow
  - Issue templates
  - PR template
  - Contributing guidelines

### Security
- ChaCha20-Poly1305 AEAD encryption
- Ed25519 signatures
- X25519 key exchange
- Multi-ratchet key derivation per packet class
- Replay protection with sliding window

## [1.0.0] - 2026-01-25

### Added
- Internal security audit completed
- Performance optimization across core paths
- Formal stability guarantees in test harness

## [0.0.1] - 2026-01-22

### Added
- Initial release as research prototype
- All core functionality implemented
- 86 tests passing across all crates

---

## Version History

| Version | Date | Status |
|---------|------|--------|
| 0.2.0 | 2024-01-15 | Production |
| 0.1.0 | 2024-01-10 | Production |
| 1.0.0 | 2026-01-25 | Future Release |
| 0.0.1 | 2026-01-22 | Research Prototype |

## Roadmap

### v0.3 - Enhanced Production Features (Planned)
- Advanced NAT traversal (TURN relay support)
- Enhanced mobile SDK features
- Additional MSP profiles
- Performance optimizations

### v0.2 - Production Readiness (Completed)
- ✅ Standardized version management
- ✅ Observability infrastructure (logging, metrics, tracing)
- ✅ Security hardening (fuzzing, auditing, SBOM)
- ✅ Performance validation (benchmarks, load testing)
- ✅ Operational tooling (health checks, alerting)

### v0.1 - Alpha (Completed)
- ✅ End-to-end integration tests
- ✅ Real network testing
- ✅ Basic NAT traversal (STUN + hole punching)
- ✅ Basic performance benchmarks

### v1.0 - Production (Future)
- Security audit (external)
- Performance optimization
- Formal stability guarantees
