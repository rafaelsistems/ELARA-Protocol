//! Cryptographic Operations Benchmarks
//!
//! Comprehensive benchmarks for ELARA cryptographic operations including:
//! - Encryption/decryption for various payload sizes
//! - Signature generation and verification
//! - Key derivation
//! - Identity generation

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use elara_bench::WIRE_PAYLOAD_SIZES;
use elara_core::{NodeId, PacketClass, RepresentationProfile, SessionId};
use elara_crypto::{Identity, SecureFrameProcessor};
use elara_wire::Extensions;

/// Benchmark identity generation
fn bench_identity_generate(c: &mut Criterion) {
    c.bench_function("crypto/identity_generate", |b| {
        b.iter(|| {
            let identity = Identity::generate();
            black_box(identity)
        })
    });
}

/// Benchmark secure frame encryption for various payload sizes
fn bench_secure_frame_encrypt(c: &mut Criterion) {
    let mut group = c.benchmark_group("crypto/encrypt");

    for &size in WIRE_PAYLOAD_SIZES {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let session_id = SessionId::new(0x1234_5678_9ABC_DEF0);
            let node_id = NodeId::new(0xDEAD_BEEF_CAFE_BABE);
            let session_key = [0x42u8; 32];
            let mut processor = SecureFrameProcessor::new(session_id, node_id, session_key);
            let payload = vec![0u8; size];

            b.iter(|| {
                processor
                    .encrypt_frame(
                        black_box(PacketClass::Core),
                        black_box(RepresentationProfile::Textual),
                        black_box(0),
                        black_box(Extensions::new()),
                        black_box(&payload),
                    )
                    .unwrap()
            });
        });
    }

    group.finish();
}

/// Benchmark secure frame decryption for various payload sizes
fn bench_secure_frame_decrypt(c: &mut Criterion) {
    let mut group = c.benchmark_group("crypto/decrypt");

    for &size in WIRE_PAYLOAD_SIZES {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let session_id = SessionId::new(0x1234_5678_9ABC_DEF0);
            let node_id = NodeId::new(0xDEAD_BEEF_CAFE_BABE);
            let session_key = [0x42u8; 32];

            let mut encrypt_processor = SecureFrameProcessor::new(session_id, node_id, session_key);
            let payload = vec![0u8; size];
            let encrypted = encrypt_processor
                .encrypt_frame(
                    PacketClass::Core,
                    RepresentationProfile::Textual,
                    0,
                    Extensions::new(),
                    &payload,
                )
                .unwrap();

            b.iter(|| {
                let mut decrypt_processor =
                    SecureFrameProcessor::new(session_id, node_id, session_key);
                decrypt_processor.decrypt_frame(black_box(&encrypted)).unwrap()
            });
        });
    }

    group.finish();
}

/// Benchmark encryption/decryption roundtrip
fn bench_encrypt_decrypt_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("crypto/roundtrip");

    for &size in WIRE_PAYLOAD_SIZES {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let session_id = SessionId::new(0x1234_5678_9ABC_DEF0);
            let node_id = NodeId::new(0xDEAD_BEEF_CAFE_BABE);
            let session_key = [0x42u8; 32];
            let payload = vec![0u8; size];

            b.iter(|| {
                let mut encrypt_processor =
                    SecureFrameProcessor::new(session_id, node_id, session_key);
                let encrypted = encrypt_processor
                    .encrypt_frame(
                        PacketClass::Core,
                        RepresentationProfile::Textual,
                        0,
                        Extensions::new(),
                        black_box(&payload),
                    )
                    .unwrap();

                let mut decrypt_processor =
                    SecureFrameProcessor::new(session_id, node_id, session_key);
                let decrypted = decrypt_processor.decrypt_frame(&encrypted).unwrap();
                black_box(decrypted)
            });
        });
    }

    group.finish();
}

/// Benchmark signature generation (via identity)
fn bench_signature_generation(c: &mut Criterion) {
    let identity = Identity::generate();
    let message = b"Test message for signature generation benchmark";

    c.bench_function("crypto/sign", |b| {
        b.iter(|| {
            let signature = identity.sign(black_box(message));
            black_box(signature)
        });
    });
}

/// Benchmark signature verification
fn bench_signature_verification(c: &mut Criterion) {
    let identity = Identity::generate();
    let message = b"Test message for signature verification benchmark";
    let signature = identity.sign(message);

    c.bench_function("crypto/verify", |b| {
        b.iter(|| {
            let valid = identity.verify(black_box(message), black_box(&signature));
            black_box(valid)
        });
    });
}

/// Benchmark key derivation (using identity generation as proxy)
fn bench_key_derivation(c: &mut Criterion) {
    c.bench_function("crypto/key_derivation", |b| {
        b.iter(|| {
            // Use identity generation as a proxy for key derivation
            let identity = Identity::generate();
            black_box(identity.node_id())
        });
    });
}

/// Benchmark session key generation (using random bytes)
fn bench_session_key_generation(c: &mut Criterion) {
    use rand::RngCore;
    
    c.bench_function("crypto/session_key_gen", |b| {
        b.iter(|| {
            let mut session_key = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut session_key);
            black_box(session_key)
        });
    });
}

criterion_group!(
    benches,
    bench_identity_generate,
    bench_secure_frame_encrypt,
    bench_secure_frame_decrypt,
    bench_encrypt_decrypt_roundtrip,
    bench_signature_generation,
    bench_signature_verification,
    bench_key_derivation,
    bench_session_key_generation,
);
criterion_main!(benches);
