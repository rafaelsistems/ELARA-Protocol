# ELARA Protocol Fuzzing

This directory contains cargo-fuzz targets for the ELARA Protocol.

## Prerequisites

Install cargo-fuzz:

```bash
cargo install cargo-fuzz
```

**Note**: Fuzzing with sanitizers works best on Linux and macOS. On Windows, you may encounter DLL issues with the address sanitizer. For Windows development, consider using WSL2 or running fuzzing in CI.

## Available Fuzz Targets

1. **wire_protocol**: Fuzzes wire protocol frame parsing
   - Tests: Frame::parse() with arbitrary bytes
   - Goal: Find panics, buffer overflows, or unexpected errors

2. **crypto_operations**: Fuzzes cryptographic operations
   - Tests: Encryption/decryption roundtrips with arbitrary inputs
   - Goal: Find crypto implementation bugs or edge cases

3. **state_reconciliation**: Fuzzes state reconciliation logic
   - Tests: State merge operations with arbitrary event sequences
   - Goal: Find state inconsistencies or merge bugs

## Running Fuzz Tests

### List all targets

```bash
cargo fuzz list
```

### Run a specific target

```bash
# Run for 60 seconds
cargo fuzz run wire_protocol -- -max_total_time=60

# Run with 4 parallel jobs
cargo fuzz run wire_protocol -- -jobs=4

# Run with custom timeout per test case (1 second)
cargo fuzz run wire_protocol -- -timeout=1
```

### Run all targets

```bash
for target in $(cargo fuzz list); do
    echo "Fuzzing $target..."
    cargo fuzz run $target -- -max_total_time=300
done
```

## Corpus Management

Fuzz corpora are stored in `corpus/<target_name>/`. These contain interesting test cases discovered during fuzzing.

To add a seed corpus:

```bash
# Create corpus directory
mkdir -p corpus/wire_protocol

# Add seed files
echo "test data" > corpus/wire_protocol/seed1
```

## Crash Artifacts

When a crash is found, it's saved to `artifacts/<target_name>/`. To reproduce:

```bash
# Reproduce a crash
cargo fuzz run wire_protocol artifacts/wire_protocol/crash-<hash>
```

## CI Integration

Fuzzing runs nightly in CI for 8 hours per target. See `.github/workflows/fuzz.yml`.

## Performance Tips

1. **Use release mode**: Fuzzing is much faster in release mode (default)
2. **Parallel jobs**: Use `-jobs=N` to utilize multiple cores
3. **Corpus minimization**: Periodically minimize corpus with `cargo fuzz cmin`
4. **Dictionary**: Add a dictionary file for structured input formats

## Troubleshooting

### "error: no such subcommand: `fuzz`"

Install cargo-fuzz: `cargo install cargo-fuzz`

### Slow fuzzing performance

- Ensure you're running in release mode
- Use multiple jobs: `-jobs=4`
- Check CPU governor: `cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor`
  - Should be "performance" not "powersave"

### Out of memory

Reduce parallel jobs or add memory limits: `-rss_limit_mb=2048`

## Resources

- [cargo-fuzz documentation](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [libFuzzer documentation](https://llvm.org/docs/LibFuzzer.html)
- [ELARA fuzzing design](../docs/specs/production-readiness-implementation/design.md)
