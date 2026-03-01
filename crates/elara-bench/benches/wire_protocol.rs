//! Wire Protocol Benchmarks
//!
//! Comprehensive benchmarks for ELARA wire protocol operations including:
//! - Frame encoding for various payload sizes (64B to 16KB)
//! - Frame decoding
//! - Packet serialization/deserialization
//! - Header parsing and serialization

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use elara_bench::WIRE_PAYLOAD_SIZES;
use elara_core::{NodeId, PacketClass, SessionId};
use elara_wire::{Extensions, FixedHeader, Frame, FrameBuilder, FIXED_HEADER_SIZE};

/// Benchmark frame encoding for various payload sizes
fn bench_frame_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("wire/frame_encoding");

    for &size in WIRE_PAYLOAD_SIZES {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let header = FixedHeader::new(
                SessionId::new(0xDEADBEEF_CAFEBABE),
                NodeId::new(0x12345678_9ABCDEF0),
            );
            let payload = vec![0u8; size];
            let mut ext = Extensions::new();
            ext.ratchet_id = Some(42);
            ext.key_epoch = Some(1);

            b.iter(|| {
                let frame = FrameBuilder::new(header.clone())
                    .extensions(ext.clone())
                    .payload(black_box(payload.clone()))
                    .auth_tag([0xAA; 16])
                    .build();
                black_box(frame.serialize())
            });
        });
    }

    group.finish();
}

/// Benchmark frame decoding for various payload sizes
fn bench_frame_decoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("wire/frame_decoding");

    for &size in WIRE_PAYLOAD_SIZES {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let header = FixedHeader::new(
                SessionId::new(0xDEADBEEF_CAFEBABE),
                NodeId::new(0x12345678_9ABCDEF0),
            );
            let payload = vec![0u8; size];
            let mut ext = Extensions::new();
            ext.ratchet_id = Some(42);
            ext.key_epoch = Some(1);

            let frame = FrameBuilder::new(header)
                .extensions(ext)
                .payload(payload)
                .auth_tag([0xAA; 16])
                .build();

            let bytes = frame.serialize().unwrap();

            b.iter(|| {
                let parsed = Frame::parse(black_box(&bytes));
                black_box(parsed)
            });
        });
    }

    group.finish();
}

/// Benchmark header parsing
fn bench_header_parse(c: &mut Criterion) {
    let header = FixedHeader {
        session_id: SessionId::new(0xDEADBEEF_CAFEBABE),
        node_id: NodeId::new(0x12345678_9ABCDEF0),
        class: PacketClass::Perceptual,
        time_hint: 12345,
        ..Default::default()
    };
    let bytes = header.to_bytes();

    c.bench_function("wire/header_parse", |b| {
        b.iter(|| FixedHeader::parse(black_box(&bytes)))
    });
}

/// Benchmark header serialization
fn bench_header_serialize(c: &mut Criterion) {
    let header = FixedHeader {
        session_id: SessionId::new(0xDEADBEEF_CAFEBABE),
        node_id: NodeId::new(0x12345678_9ABCDEF0),
        class: PacketClass::Perceptual,
        time_hint: 12345,
        ..Default::default()
    };

    c.bench_function("wire/header_serialize", |b| {
        let mut buf = [0u8; FIXED_HEADER_SIZE];
        b.iter(|| header.serialize(black_box(&mut buf)))
    });
}

/// Benchmark complete frame roundtrip (serialize + parse)
fn bench_frame_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("wire/frame_roundtrip");

    for &size in WIRE_PAYLOAD_SIZES {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let header = FixedHeader::new(SessionId::new(12345), NodeId::new(67890));
            let mut ext = Extensions::new();
            ext.ratchet_id = Some(42);
            ext.key_epoch = Some(1);
            let payload = vec![0u8; size];

            b.iter(|| {
                let frame = FrameBuilder::new(header.clone())
                    .extensions(ext.clone())
                    .payload(black_box(payload.clone()))
                    .auth_tag([0xAA; 16])
                    .build();

                let bytes = frame.serialize().unwrap();
                let parsed = Frame::parse(black_box(&bytes)).unwrap();
                black_box(parsed)
            });
        });
    }

    group.finish();
}

/// Benchmark extensions encoding
fn bench_extensions_encoding(c: &mut Criterion) {
    let mut ext = Extensions::new();
    ext.ratchet_id = Some(42);
    ext.key_epoch = Some(1);
    ext.interest_mask = Some(0xFFFF_FFFF_FFFF_FFFF);

    c.bench_function("wire/extensions_encode", |b| {
        b.iter(|| {
            // Extensions are encoded as part of frame serialization
            // Just measure the overhead of cloning
            let cloned = black_box(&ext).clone();
            black_box(cloned)
        });
    });
}

criterion_group!(
    benches,
    bench_frame_encoding,
    bench_frame_decoding,
    bench_header_parse,
    bench_header_serialize,
    bench_frame_roundtrip,
    bench_extensions_encoding,
);
criterion_main!(benches);
