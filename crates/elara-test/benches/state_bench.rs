//! Benchmarks for ELARA state operations

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use elara_core::{NodeId, VersionVector};

fn bench_version_vector_increment(c: &mut Criterion) {
    let mut vv = VersionVector::new();
    let node = NodeId::new(1);

    c.bench_function("version_vector_increment", |b| {
        b.iter(|| {
            vv.increment(black_box(node));
        })
    });
}

fn bench_version_vector_merge(c: &mut Criterion) {
    let mut vv1 = VersionVector::new();
    let mut vv2 = VersionVector::new();

    // Setup with some entries
    for i in 0..10 {
        vv1.increment(NodeId::new(i));
        vv2.increment(NodeId::new(i + 5));
    }

    c.bench_function("version_vector_merge", |b| {
        b.iter(|| black_box(vv1.merge(black_box(&vv2))))
    });
}

fn bench_version_vector_happens_before(c: &mut Criterion) {
    let mut vv1 = VersionVector::new();
    let mut vv2 = VersionVector::new();

    for i in 0..10 {
        vv1.increment(NodeId::new(i));
        vv2.increment(NodeId::new(i));
    }
    vv2.increment(NodeId::new(0));

    c.bench_function("version_vector_happens_before", |b| {
        b.iter(|| black_box(vv1.happens_before(black_box(&vv2))))
    });
}

fn bench_version_vector_concurrent(c: &mut Criterion) {
    let mut vv1 = VersionVector::new();
    let mut vv2 = VersionVector::new();

    for i in 0..5 {
        vv1.increment(NodeId::new(i));
    }
    for i in 5..10 {
        vv2.increment(NodeId::new(i));
    }

    c.bench_function("version_vector_concurrent", |b| {
        b.iter(|| {
            // Check if neither dominates the other (concurrent)
            let hb1 = vv1.happens_before(black_box(&vv2));
            let hb2 = vv2.happens_before(black_box(&vv1));
            black_box(!hb1 && !hb2)
        })
    });
}

fn bench_version_vector_get(c: &mut Criterion) {
    let mut vv = VersionVector::new();

    for i in 0..100 {
        vv.increment(NodeId::new(i));
    }

    c.bench_function("version_vector_get", |b| {
        b.iter(|| black_box(vv.get(black_box(NodeId::new(50)))))
    });
}

criterion_group!(
    benches,
    bench_version_vector_increment,
    bench_version_vector_merge,
    bench_version_vector_happens_before,
    bench_version_vector_concurrent,
    bench_version_vector_get,
);
criterion_main!(benches);
