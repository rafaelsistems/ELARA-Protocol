# elara-fuzz

Fuzzing infrastructure for the ELARA Protocol.

## Overview

This crate provides a trait-based framework for creating fuzz targets that can discover edge cases, panics, and security vulnerabilities in parsing and cryptographic code.

The fuzzing infrastructure is designed to integrate seamlessly with cargo-fuzz and libfuzzer for production-grade fuzzing campaigns.

## Features

- **FuzzTarget trait**: Define custom fuzz targets with arbitrary input types
- **FuzzResult enum**: Classify fuzz outcomes (Ok, Bug, Invalid)
- **cargo-fuzz integration**: Compatible with libfuzzer-sys for production fuzzing
- **Pre-built fuzzers**: Ready-to-use fuzzers for wire protocol, crypto, and state reconciliation

## Architecture

The fuzzing framework consists of three main components:

1. **Core Trait (`FuzzTarget`)**: Defines the interface for all fuzz targets
2. **Result Classification (`FuzzResult`)**: Categorizes fuzzing outcomes
3. **Concrete Implementations**: Specific fuzzers for ELARA components

## Usage

### Implementing a Custom Fuzzer

Implement the `FuzzTarget` trait for your fuzzer:

```rust
use elara_fuzz::{FuzzTarget, FuzzResult};
use arbitrary::Arbitrary;

#[derive(Arbitrary, Debug)]
struct MyInput {
    data: Vec<u8>,
}

struct MyFuzzer {
    // Your fuzzer state
}

impl FuzzTarget for MyFuzzer {
    type Input = MyInput;

    fn fuzz_once(&mut self, input: Self::Input) -> FuzzResult {
        // Test your code with arbitrary input
        match process_data(&input.data) {
            Ok(_) => FuzzResult::Ok,
            Err(e) if e.is_expected() => FuzzResult::Invalid,
            Err(e) => FuzzResult::Bug(format!("Unexpected error: {}", e)),
        }
    }
}
```

### Integration with cargo-fuzz

Create fuzz targets in the `fuzz/fuzz_targets/` directory:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use elara_fuzz::{FuzzTarget, MyFuzzer};

fuzz_target!(|data: <MyFuzzer as FuzzTarget>::Input| {
    let mut fuzzer = MyFuzzer::new();
    let _ = fuzzer.fuzz_once(data);
});
```

### Running Fuzz Tests

```bash
# Install cargo-fuzz
cargo install cargo-fuzz

# List available fuzz targets
cargo fuzz list

# Run a specific fuzz target
cargo fuzz run wire_protocol

# Run with specific options
cargo fuzz run wire_protocol -- -max_total_time=3600 -jobs=4
```

## Pre-built Fuzzers

The crate includes fuzz targets for:

1. **Wire Protocol Fuzzer** (`wire_protocol`): Tests frame parsing with arbitrary bytes
2. **Crypto Operations Fuzzer** (`crypto_operations`): Tests encryption/decryption roundtrips
3. **State Reconciliation Fuzzer** (`state_reconciliation`): Tests state merge operations

## CI Integration

Fuzzing is integrated into the CI pipeline with nightly 8-hour fuzzing runs. See `.github/workflows/fuzz.yml` for configuration.

## Corpus Management

Fuzz corpora are stored in `fuzz/corpus/<target_name>/`. Interesting test cases discovered during fuzzing are automatically added to the corpus for regression testing.

## Crash Reporting

When a crash is discovered:
1. The crashing input is saved to `fuzz/artifacts/<target_name>/`
2. A detailed crash report is generated
3. The fuzzing run is marked as failed
4. Developers are notified to investigate and fix

## Performance

The fuzzing infrastructure is designed for high throughput:
- Target: 10,000+ executions per second per core
- Parallel execution across multiple cores
- Efficient corpus minimization

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
