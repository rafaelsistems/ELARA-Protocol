# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
| 1.0.0 | 2026-01-25 | Production |
| 0.0.1 | 2026-01-22 | Research Prototype |

## Roadmap

### v0.1 - Alpha (Completed)
- End-to-end integration tests
- Real network testing
- Basic NAT traversal (STUN + hole punching)
- Basic performance benchmarks

### v0.2 - Beta (Planned)
- Voice codec integration
- Mobile SDK (Android/iOS)
- Chaos testing in real networks

### v1.0 - Production (Completed)
- Security audit (internal)
- Performance optimization
- Formal stability guarantees
