//! Benchmarks for ELARA cryptographic operations

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

use elara_core::{NodeId, PacketClass, RepresentationProfile, SessionId};
use elara_crypto::{Identity, SecureFrameProcessor};
use elara_wire::Extensions;

fn bench_identity_generate(c: &mut Criterion) {
    c.bench_function("identity_generate", |b| {
        b.iter(|| {
            let identity = Identity::generate();
            black_box(identity)
        })
    });
}

fn bench_secure_frame_encrypt(c: &mut Criterion) {
    let session_id = SessionId::new(0x1234_5678_9ABC_DEF0);
    let node_id = NodeId::new(0xDEAD_BEEF_CAFE_BABE);
    let session_key = [0x42u8; 32];
    let mut processor = SecureFrameProcessor::new(session_id, node_id, session_key);

    let payload = vec![0u8; 256];

    let mut group = c.benchmark_group("secure_frame_encrypt");
    group.throughput(Throughput::Bytes(256));

    group.bench_function("256_bytes", |b| {
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
        })
    });

    group.finish();
}

fn bench_secure_frame_encrypt_sizes(c: &mut Criterion) {
    let session_id = SessionId::new(0x1234_5678_9ABC_DEF0);
    let node_id = NodeId::new(0xDEAD_BEEF_CAFE_BABE);
    let session_key = [0x42u8; 32];

    let sizes = [64, 256, 1024, 4096];

    let mut group = c.benchmark_group("encrypt_by_size");

    for size in sizes {
        let mut processor = SecureFrameProcessor::new(session_id, node_id, session_key);
        let payload = vec![0u8; size];

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_function(format!("{}_bytes", size), |b| {
            b.iter(|| {
                processor
                    .encrypt_frame(
                        PacketClass::Core,
                        RepresentationProfile::Textual,
                        0,
                        Extensions::new(),
                        black_box(&payload),
                    )
                    .unwrap()
            })
        });
    }

    group.finish();
}

fn bench_secure_frame_decrypt(c: &mut Criterion) {
    let session_id = SessionId::new(0x1234_5678_9ABC_DEF0);
    let node_id = NodeId::new(0xDEAD_BEEF_CAFE_BABE);
    let session_key = [0x42u8; 32];

    let mut encrypt_processor = SecureFrameProcessor::new(session_id, node_id, session_key);
    let mut decrypt_processor = SecureFrameProcessor::new(session_id, node_id, session_key);

    let payload = vec![0u8; 256];
    let encrypted = encrypt_processor
        .encrypt_frame(
            PacketClass::Core,
            RepresentationProfile::Textual,
            0,
            Extensions::new(),
            &payload,
        )
        .unwrap();

    let mut group = c.benchmark_group("secure_frame_decrypt");
    group.throughput(Throughput::Bytes(256));

    group.bench_function("256_bytes", |b| {
        b.iter(|| {
            decrypt_processor
                .decrypt_frame(black_box(&encrypted))
                .unwrap()
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_identity_generate,
    bench_secure_frame_encrypt,
    bench_secure_frame_encrypt_sizes,
    bench_secure_frame_decrypt,
);
criterion_main!(benches);
