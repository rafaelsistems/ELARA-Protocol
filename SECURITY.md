# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.x     | :white_check_mark: |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them via email to the maintainers or through GitHub's private vulnerability reporting feature.

### What to Include

- Type of issue (e.g., buffer overflow, cryptographic weakness, replay attack)
- Full paths of source file(s) related to the issue
- Location of the affected source code (tag/branch/commit or direct URL)
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the issue, including how an attacker might exploit it

### Response Timeline

- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Resolution Target**: Within 30 days for critical issues

## Security Model

### Threat Model

ELARA is designed to protect against:

- **Passive eavesdropping** - All payloads are encrypted
- **Message replay** - Per-class replay windows
- **Message modification** - AEAD authentication
- **Server-side content access** - Server blindness by design
- **Cross-session correlation** - Session key isolation

ELARA does **not** protect against:

- **Endpoint compromise** - If your device is compromised, keys are exposed
- **Traffic analysis** - Metadata (timing, size) is visible
- **Denial of service** - Resource exhaustion attacks
- **Quantum computers** - Current crypto is not post-quantum

### Cryptographic Choices

| Purpose | Algorithm | Notes |
|---------|-----------|-------|
| Signing | Ed25519 | Well-audited, fast |
| Key Exchange | X25519 | Curve25519 ECDH |
| AEAD | ChaCha20-Poly1305 | Mobile-friendly, constant-time |
| KDF | HKDF-SHA256 | Standard derivation |

### Known Limitations

1. **No post-quantum security** - Planned for future versions
2. **No formal verification** - Implementation not formally verified
3. **No third-party audit** - Internal audit completed

## Security Best Practices for Contributors

### Code Review

- All cryptographic code changes require review
- No custom cryptographic implementations
- Use well-audited libraries only

### Testing

- Property-based tests for crypto operations
- Fuzzing for parsing code
- Replay protection tests

### Dependencies

- Minimal dependencies
- Pin dependency versions
- Regular security updates

## Acknowledgments

We appreciate responsible disclosure of security issues.
