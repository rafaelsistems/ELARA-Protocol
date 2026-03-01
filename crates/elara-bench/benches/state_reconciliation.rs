//! State Reconciliation Benchmarks
//!
//! Comprehensive benchmarks for ELARA state operations including:
//! - State merge for various event counts
//! - Version vector operations
//! - Causality checking (happens-before, concurrent)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use elara_bench::EVENT_COUNTS;
use elara_core::{NodeId, VersionVector};

/// Benchmark version vector increment operation
fn bench_version_vector_increment(c: &mut Criterion) {
    let mut vv = VersionVector::new();
    let node = NodeId::new(1);

    c.bench_function("state/vv_increment", |b| {
        b.iter(|| {
            vv.increment(black_box(node));
        })
    });
}

/// Benchmark version vector get operation
fn bench_version_vector_get(c: &mut Criterion) {
    let mut vv = VersionVector::new();

    // Setup with 100 entries
    for i in 0..100 {
        vv.increment(NodeId::new(i));
    }

    c.bench_function("state/vv_get", |b| {
        b.iter(|| black_box(vv.get(black_box(NodeId::new(50)))))
    });
}

/// Benchmark version vector merge for various sizes
fn bench_version_vector_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("state/vv_merge");

    for &count in EVENT_COUNTS {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let mut vv1 = VersionVector::new();
            let mut vv2 = VersionVector::new();

            // Setup with overlapping entries
            for i in 0..count {
                vv1.increment(NodeId::new(i as u64));
                vv2.increment(NodeId::new((i + count / 2) as u64));
            }

            b.iter(|| black_box(vv1.merge(black_box(&vv2))))
        });
    }

    group.finish();
}

/// Benchmark version vector happens-before check
fn bench_version_vector_happens_before(c: &mut Criterion) {
    let mut group = c.benchmark_group("state/vv_happens_before");

    for &count in EVENT_COUNTS {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let mut vv1 = VersionVector::new();
            let mut vv2 = VersionVector::new();

            for i in 0..count {
                vv1.increment(NodeId::new(i as u64));
                vv2.increment(NodeId::new(i as u64));
            }
            // Make vv2 strictly after vv1
            vv2.increment(NodeId::new(0));

            b.iter(|| black_box(vv1.happens_before(black_box(&vv2))))
        });
    }

    group.finish();
}

/// Benchmark version vector concurrent check
fn bench_version_vector_concurrent(c: &mut Criterion) {
    let mut group = c.benchmark_group("state/vv_concurrent");

    for &count in EVENT_COUNTS {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let mut vv1 = VersionVector::new();
            let mut vv2 = VersionVector::new();

            // Create concurrent version vectors (disjoint updates)
            for i in 0..(count / 2) {
                vv1.increment(NodeId::new(i as u64));
            }
            for i in (count / 2)..count {
                vv2.increment(NodeId::new(i as u64));
            }

            b.iter(|| {
                // Check if neither dominates the other (concurrent)
                let hb1 = vv1.happens_before(black_box(&vv2));
                let hb2 = vv2.happens_before(black_box(&vv1));
                black_box(!hb1 && !hb2)
            });
        });
    }

    group.finish();
}

/// Benchmark version vector equality check
fn bench_version_vector_equals(c: &mut Criterion) {
    let mut group = c.benchmark_group("state/vv_equals");

    for &count in EVENT_COUNTS {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let mut vv1 = VersionVector::new();
            let mut vv2 = VersionVector::new();

            for i in 0..count {
                vv1.increment(NodeId::new(i as u64));
                vv2.increment(NodeId::new(i as u64));
            }

            b.iter(|| black_box(&vv1 == &vv2))
        });
    }

    group.finish();
}

/// Benchmark version vector clone
fn bench_version_vector_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("state/vv_clone");

    for &count in EVENT_COUNTS {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let mut vv = VersionVector::new();

            for i in 0..count {
                vv.increment(NodeId::new(i as u64));
            }

            b.iter(|| {
                let cloned = black_box(&vv).clone();
                black_box(cloned)
            });
        });
    }

    group.finish();
}

/// Benchmark causality determination (full workflow)
fn bench_causality_determination(c: &mut Criterion) {
    let mut group = c.benchmark_group("state/causality_check");

    for &count in EVENT_COUNTS {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let mut vv1 = VersionVector::new();
            let mut vv2 = VersionVector::new();

            for i in 0..count {
                vv1.increment(NodeId::new(i as u64));
                vv2.increment(NodeId::new(i as u64));
            }
            vv2.increment(NodeId::new(0));

            b.iter(|| {
                // Determine relationship: before, after, concurrent, or equal
                let v1_before_v2 = vv1.happens_before(black_box(&vv2));
                let v2_before_v1 = vv2.happens_before(black_box(&vv1));
                let equal = vv1 == vv2;

                let relationship = if equal {
                    "equal"
                } else if v1_before_v2 {
                    "before"
                } else if v2_before_v1 {
                    "after"
                } else {
                    "concurrent"
                };

                black_box(relationship)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_version_vector_increment,
    bench_version_vector_get,
    bench_version_vector_merge,
    bench_version_vector_happens_before,
    bench_version_vector_concurrent,
    bench_version_vector_equals,
    bench_version_vector_clone,
    bench_causality_determination,
);
criterion_main!(benches);
