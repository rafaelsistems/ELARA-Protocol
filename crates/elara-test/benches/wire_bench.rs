//! Benchmarks for ELARA wire protocol

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use elara_core::{NodeId, PacketClass, SessionId};
use elara_wire::{Extensions, FixedHeader, Frame, FrameBuilder, FIXED_HEADER_SIZE};

fn bench_header_parse(c: &mut Criterion) {
    let header = FixedHeader {
        session_id: SessionId::new(0xDEADBEEF_CAFEBABE),
        node_id: NodeId::new(0x12345678_9ABCDEF0),
        class: PacketClass::Perceptual,
        time_hint: 12345,
        ..Default::default()
    };
    let bytes = header.to_bytes();

    c.bench_function("header_parse", |b| {
        b.iter(|| FixedHeader::parse(black_box(&bytes)))
    });
}

fn bench_header_serialize(c: &mut Criterion) {
    let header = FixedHeader {
        session_id: SessionId::new(0xDEADBEEF_CAFEBABE),
        node_id: NodeId::new(0x12345678_9ABCDEF0),
        class: PacketClass::Perceptual,
        time_hint: 12345,
        ..Default::default()
    };

    c.bench_function("header_serialize", |b| {
        let mut buf = [0u8; FIXED_HEADER_SIZE];
        b.iter(|| header.serialize(black_box(&mut buf)))
    });
}

fn bench_frame_roundtrip(c: &mut Criterion) {
    let header = FixedHeader::new(SessionId::new(12345), NodeId::new(67890));
    let mut ext = Extensions::new();
    ext.ratchet_id = Some(42);
    ext.key_epoch = Some(1);

    let frame = FrameBuilder::new(header)
        .extensions(ext)
        .payload(vec![1, 2, 3, 4, 5, 6, 7, 8])
        .auth_tag([0xAA; 16])
        .build();

    let bytes = frame.serialize().unwrap();

    c.bench_function("frame_roundtrip", |b| {
        b.iter(|| {
            let parsed = Frame::parse(black_box(&bytes)).unwrap();
            black_box(parsed)
        })
    });
}

criterion_group!(benches, bench_header_parse, bench_header_serialize, bench_frame_roundtrip);
criterion_main!(benches);
