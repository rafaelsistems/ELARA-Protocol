# Contributing to ELARA Protocol

Thank you for your interest in contributing to ELARA! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Testing Guidelines](#testing-guidelines)
- [Documentation](#documentation)
- [Areas for Contribution](#areas-for-contribution)

## Code of Conduct

This project adheres to a code of conduct. By participating, you are expected to:

- Be respectful and inclusive
- Accept constructive criticism gracefully
- Focus on what is best for the community
- Show empathy towards other community members

## Getting Started

### Prerequisites

- **Rust 1.70+** - [Install Rust](https://rustup.rs/)
- **Git** - For version control
- **Cargo** - Comes with Rust

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork:

```bash
git clone https://github.com/YOUR_USERNAME/ELARA-Protocol.git
cd ELARA-Protocol
```

3. Add upstream remote:

```bash
git remote add upstream https://github.com/rafaelsistems/ELARA-Protocol.git
```

## Development Setup

### Install Development Tools

```bash
# Recommended tools
cargo install cargo-watch    # Auto-rebuild on changes
cargo install cargo-nextest  # Faster test runner
cargo install cargo-clippy   # Linter
cargo install cargo-fmt      # Formatter
```

### Build and Test

```bash
# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace

# Run tests with output
cargo test --workspace -- --nocapture

# Run specific crate tests
cargo test -p elara-core
cargo test -p elara-crypto

# Watch mode (auto-run tests on change)
cargo watch -x "test --workspace"
```

### Generate Documentation

```bash
cargo doc --workspace --no-deps --open
```

## How to Contribute

### Reporting Bugs

1. Check existing [issues](https://github.com/rafaelsistems/ELARA-Protocol/issues)
2. Create a new issue with:
   - Clear title
   - Steps to reproduce
   - Expected vs actual behavior
   - Environment details (OS, Rust version)

### Suggesting Features

1. Open a [discussion](https://github.com/rafaelsistems/ELARA-Protocol/discussions)
2. Describe the feature and use case
3. Wait for feedback before implementing

### Submitting Code

1. Create a branch from `main`:

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/your-bug-fix
```

2. Make your changes
3. Write/update tests
4. Run the test suite
5. Submit a pull request

## Pull Request Process

### Before Submitting

- [ ] Code compiles without warnings: `cargo build --workspace`
- [ ] All tests pass: `cargo test --workspace`
- [ ] Code is formatted: `cargo fmt --all`
- [ ] No clippy warnings: `cargo clippy --workspace -- -D warnings`
- [ ] Documentation updated if needed
- [ ] Commit messages are clear

### PR Title Format

```
type(scope): description

Examples:
feat(crypto): add post-quantum key exchange
fix(time): correct horizon adaptation calculation
docs(readme): update installation instructions
test(state): add partition merge tests
refactor(wire): simplify frame parsing
```

### PR Description Template

```markdown
## Summary
Brief description of changes

## Changes
- Change 1
- Change 2

## Testing
How was this tested?

## Related Issues
Fixes #123
```

## Coding Standards

### Rust Style

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting

### Naming Conventions

```rust
// Types: PascalCase
struct StateAtom { }
enum PacketClass { }

// Functions/methods: snake_case
fn process_event() { }
impl StateField {
    fn get_atom(&self) { }
}

// Constants: SCREAMING_SNAKE_CASE
const MAX_FRAME_SIZE: usize = 1200;

// Modules: snake_case
mod time_engine;
```

### Documentation

```rust
/// Brief description of the function.
///
/// More detailed explanation if needed.
///
/// # Arguments
///
/// * `param1` - Description of param1
/// * `param2` - Description of param2
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// When and why this function returns errors
///
/// # Examples
///
/// ```rust
/// let result = my_function(arg1, arg2);
/// ```
pub fn my_function(param1: Type1, param2: Type2) -> Result<Output, Error> {
    // ...
}
```

### Error Handling

```rust
// Use ElaraError for all errors
use elara_core::{ElaraError, ElaraResult};

// Return Result, don't panic
pub fn parse_frame(data: &[u8]) -> ElaraResult<Frame> {
    if data.len() < HEADER_SIZE {
        return Err(ElaraError::InvalidFrame);
    }
    // ...
}
```

## Testing Guidelines

### Test Organization

```rust
// Unit tests in the same file
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        // Arrange
        let input = create_test_input();
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected);
    }
}
```

### Test Naming

```rust
#[test]
fn test_<function>_<scenario>_<expected_result>() {
    // Example:
    // test_parse_frame_valid_header_returns_frame
    // test_encrypt_empty_payload_succeeds
    // test_replay_window_duplicate_seq_rejects
}
```

### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn roundtrip_serialization(data in any::<Vec<u8>>()) {
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }
}
```

### Test Coverage Goals

- Line coverage: >80%
- Branch coverage: >70%
- Critical paths: 100%

## Documentation

### When to Update Docs

- New public API
- Changed behavior
- New features
- Bug fixes that affect usage

### Documentation Locations

| Type | Location |
|------|----------|
| API docs | Inline rustdoc |
| Architecture | `docs/architecture/` |
| Specifications | `docs/specs/` |
| Implementation | `docs/implementation/` |
| Examples | `examples/` |

## Areas for Contribution

### 🧪 Testing (Good First Issues)

- Add more unit tests
- Property-based tests for CRDTs
- Chaos testing scenarios
- Benchmark suite

### 📚 Documentation

- Improve API documentation
- Add code examples
- Translate documentation
- Tutorial/guide writing

### 🔧 Implementation

- Bug fixes
- Performance optimizations
- Code cleanup/refactoring

### 🌐 Transport Layer

- STUN client implementation
- TURN relay support
- QUIC transport
- WebRTC data channel

### 📱 Platform Support

- Kotlin bindings (Android)
- Swift bindings (iOS)
- WASM support (Web)
- C FFI layer

### 🔬 Research

- Post-quantum cryptography
- New CRDT types
- Compression algorithms
- Voice codec integration

## Getting Help

- **Questions**: [GitHub Discussions](https://github.com/rafaelsistems/ELARA-Protocol/discussions)
- **Bugs**: [GitHub Issues](https://github.com/rafaelsistems/ELARA-Protocol/issues)
- **Chat**: Coming soon

## Recognition

Contributors will be recognized in:
- `CONTRIBUTORS.md` file
- Release notes
- Project documentation

Thank you for contributing to ELARA! 🎉
