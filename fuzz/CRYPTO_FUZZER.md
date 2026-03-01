# Comprehensive Crypto Fuzzer

## Overview

The `crypto_operations` fuzzer is a **production-grade** fuzzing harness that comprehensively tests all cryptographic operations in the ELARA Protocol. Unlike the initial simple implementation, this fuzzer covers:

1. **Encryption/Decryption Operations**
2. **Signature Verification**
3. **Key Derivation**
4. **Secure Frame Processing**
5. **Replay Protection**
6. **Nonce Reuse Detection**

## Architecture

### Input Types

The fuzzer uses an enum-based input structure that allows libfuzzer to explore different cryptographic scenarios:

```rust
enum CryptoFuzzInput {
    EncryptDecrypt { ... },
    SignatureVerification { ... },
    KeyDerivation { ... },
    SecureFrame { ... },
    ReplayProtection { ... },
    NonceReuse { ... },
}
```

This design ensures comprehensive coverage across all crypto operations.

### Fuzzing Strategies

#### 1. Encryption/Decryption Fuzzing

**Tests:**
- Roundtrip correctness with arbitrary plaintexts (0 bytes to large)
- Various associated data sizes
- Ciphertext tampering detection
- AAD (Associated Authenticated Data) tampering detection
- Proper authentication tag verification

**Properties Validated:**
- `decrypt(encrypt(plaintext)) == plaintext`
- Tampered ciphertext is rejected
- Wrong AAD causes decryption failure
- Authentication is cryptographically sound

#### 2. Signature Verification Fuzzing

**Tests:**
- Valid signature verification
- Corrupted signature rejection
- Truncated signature rejection
- Wrong key signature rejection
- Empty message signing
- Large message signing
- PublicIdentity verification consistency

**Properties Validated:**
- Valid signatures always verify
- Corrupted signatures always fail
- Signatures from different keys don't verify
- Identity and PublicIdentity behave consistently

#### 3. Key Derivation Fuzzing

**Tests:**
- Edge case inputs (empty, very long)
- Deterministic output for same input
- Different outputs for different inputs
- Context separation

**Properties Validated:**
- Key derivation is deterministic
- Different contexts produce different keys
- Key derivation handles edge cases gracefully

#### 4. Secure Frame Processing Fuzzing

**Tests:**
- Frame encryption/decryption roundtrip
- All packet classes (Core, Perceptual, Enhancement, Cosmetic, Repair)
- Frame tampering detection
- Replay protection integration
- Sequence number handling
- Session ID validation

**Properties Validated:**
- Encrypted frames decrypt correctly
- Tampered frames are rejected
- Replay attacks are detected
- Session isolation is maintained

#### 5. Replay Protection Fuzzing

**Tests:**
- Sequence number acceptance/rejection
- Window advancement
- Out-of-order packet handling
- Sequence number wraparound
- Duplicate detection

**Properties Validated:**
- Same sequence number is never accepted twice
- Window advances correctly
- Out-of-order packets within window are accepted
- Old packets outside window are rejected

#### 6. Nonce Reuse Detection

**Tests:**
- Encrypting different plaintexts with same nonce
- Decryption correctness despite nonce reuse
- Documentation of protocol-level concerns

**Properties Validated:**
- AEAD operations remain correct (though insecure) with nonce reuse
- Documents that nonce reuse prevention is a protocol-level concern

## Running the Fuzzer

### Quick Test (5 seconds)
```bash
cd fuzz
cargo fuzz run crypto_operations -- -max_total_time=5
```

### Standard Run (1 hour)
```bash
cd fuzz
cargo fuzz run crypto_operations -- -max_total_time=3600
```

### Nightly Run (8 hours - CI configuration)
```bash
cd fuzz
cargo fuzz run crypto_operations -- -max_total_time=28800
```

### With Corpus
```bash
cd fuzz
cargo fuzz run crypto_operations corpus/crypto_operations
```

## Crash Analysis

If the fuzzer finds a crash, it will save the input to `artifacts/crypto_operations/`:

```bash
# Reproduce a crash
cargo fuzz run crypto_operations artifacts/crypto_operations/crash-<hash>

# Minimize a crash
cargo fuzz cmin crypto_operations

# Triage crashes
cargo fuzz tmin crypto_operations artifacts/crypto_operations/crash-<hash>
```

## Coverage

The fuzzer achieves comprehensive coverage of:

- **AeadCipher**: encrypt, decrypt, tampering detection
- **Identity**: sign, verify, key generation
- **PublicIdentity**: verify, key handling
- **SecureFrameProcessor**: encrypt_frame, decrypt_frame, replay protection
- **ReplayWindow**: accept, check, window advancement
- **All PacketClass variants**: Core, Perceptual, Enhancement, Cosmetic, Repair

## Integration with CI

The fuzzer is integrated into CI via `.github/workflows/fuzz.yml`:

- Runs nightly for 8 hours
- Tests all fuzz targets including crypto_operations
- Uploads crash artifacts
- Fails build on discovered bugs

## Expected Behavior

### Valid Inputs
- Should return `FuzzResult::Ok`
- No panics or crashes
- Correct cryptographic behavior

### Invalid Inputs
- Should return `FuzzResult::Invalid`
- Graceful error handling
- No panics or crashes

### Bug Detection
- Returns `FuzzResult::Bug(description)`
- Indicates a security or correctness issue
- Requires immediate investigation

## Panic Handling

The fuzzer catches all panics and converts them to `FuzzResult::Bug`:

```rust
let result = panic::catch_unwind(|| {
    // Fuzz operation
});

match result {
    Ok(fuzz_result) => fuzz_result,
    Err(panic) => FuzzResult::Bug(format!("Panic: {}", panic)),
}
```

This ensures that any unexpected panic is treated as a bug and reported.

## Performance Considerations

- Uses fixed keys for deterministic fuzzing
- Generates new Identity per fuzzer instance (not per iteration)
- Efficient input generation via `arbitrary` crate
- Minimal allocations in hot paths

## Security Properties Tested

1. **Confidentiality**: Ciphertext doesn't leak plaintext
2. **Integrity**: Tampering is detected
3. **Authentication**: Only valid signatures verify
4. **Replay Protection**: Duplicate packets are rejected
5. **Session Isolation**: Different sessions don't interfere
6. **Key Isolation**: Different keys produce different results

## Comparison to Initial Implementation

### Before (Simple)
```rust
// Only tested basic encryption roundtrip
match cipher.encrypt(nonce, plaintext, aad) {
    Ok(ct) => match cipher.decrypt(nonce, ct, aad) {
        Ok(dec) => check_equal(dec, plaintext),
        Err(_) => Bug,
    },
    Err(_) => Invalid,
}
```

### After (Production-Grade)
- 6 different fuzzing scenarios
- Tampering detection
- Signature verification with malformed inputs
- Secure frame processing with replay protection
- Edge case handling
- Panic catching
- Comprehensive property validation

## Future Enhancements

Potential additions for even more comprehensive fuzzing:

1. **Differential Fuzzing**: Compare against reference implementations
2. **Stateful Fuzzing**: Maintain fuzzer state across iterations
3. **Coverage-Guided Mutations**: Use coverage feedback for smarter mutations
4. **Symbolic Execution**: Combine with symbolic execution tools
5. **Hardware Acceleration**: Test hardware crypto implementations

## References

- [libFuzzer Documentation](https://llvm.org/docs/LibFuzzer.html)
- [cargo-fuzz Guide](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [ELARA Crypto Architecture](../../crates/elara-crypto/README.md)
- [Fuzzing Infrastructure](./README.md)
