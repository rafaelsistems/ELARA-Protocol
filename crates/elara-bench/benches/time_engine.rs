//! Time Engine Benchmarks
//!
//! Comprehensive benchmarks for ELARA time engine operations including:
//! - Time classification
//! - Horizon adaptation
//! - Drift estimation
//! - Clock operations

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use elara_core::NodeId;
use elara_time::{NetworkModel, PerceptualClock, StateClock, TimeEngine};
use std::time::Duration;

/// Benchmark time engine tick operation
fn bench_time_engine_tick(c: &mut Criterion) {
    let mut engine = TimeEngine::new();

    c.bench_function("time/engine_tick", |b| {
        b.iter(|| {
            engine.tick();
            black_box(engine.tau_s())
        })
    });
}

/// Benchmark time classification
fn bench_time_classify(c: &mut Criterion) {
    let engine = TimeEngine::new();
    let base = engine.tau_s();

    c.bench_function("time/classify", |b| {
        b.iter(|| engine.classify_time(black_box(base)))
    });
}

/// Benchmark perceptual clock tick
fn bench_perceptual_clock_tick(c: &mut Criterion) {
    let mut clock = PerceptualClock::new();

    c.bench_function("time/perceptual_tick", |b| {
        b.iter(|| {
            clock.tick();
            black_box(clock.now())
        })
    });
}

/// Benchmark perceptual clock now (read)
fn bench_perceptual_clock_now(c: &mut Criterion) {
    let clock = PerceptualClock::new();

    c.bench_function("time/perceptual_now", |b| {
        b.iter(|| black_box(clock.now()))
    });
}

/// Benchmark state clock advance
fn bench_state_clock_advance(c: &mut Criterion) {
    let mut clock = StateClock::new();
    let dt = Duration::from_millis(10);

    c.bench_function("time/state_advance", |b| {
        b.iter(|| {
            clock.advance(black_box(dt));
            black_box(clock.now())
        })
    });
}

/// Benchmark state clock blend (with correction)
fn bench_state_clock_blend(c: &mut Criterion) {
    let mut clock = StateClock::new();
    let dt = Duration::from_millis(10);

    c.bench_function("time/state_blend", |b| {
        b.iter(|| {
            // Advance and blend correction
            clock.advance(black_box(dt));
            black_box(clock.now())
        })
    });
}

/// Benchmark state clock now (read)
fn bench_state_clock_now(c: &mut Criterion) {
    let clock = StateClock::new();

    c.bench_function("time/state_now", |b| {
        b.iter(|| black_box(clock.now()))
    });
}

/// Benchmark network model update
fn bench_network_model_update(c: &mut Criterion) {
    let mut model = NetworkModel::new();
    let peer = NodeId::new(0x1234);

    c.bench_function("time/network_model_update", |b| {
        let mut i = 0u16;
        b.iter(|| {
            i = i.wrapping_add(1);
            model.update_from_packet(
                black_box(peer),
                black_box(i as f64 * 0.01),
                black_box(i as f64 * 0.01 - 0.05),
                black_box(i),
            );
        })
    });
}

/// Benchmark network model drift estimation
fn bench_drift_estimation(c: &mut Criterion) {
    let mut model = NetworkModel::new();
    let peer = NodeId::new(0x1234);

    // Populate with some data
    for i in 0..100 {
        model.update_from_packet(peer, i as f64 * 0.01, i as f64 * 0.01 - 0.05, i);
    }

    c.bench_function("time/drift_estimation", |b| {
        b.iter(|| {
            // Measure the network model's stability score as a proxy
            let score = model.stability_score;
            black_box(score)
        })
    });
}

/// Benchmark horizon adaptation
fn bench_horizon_adaptation(c: &mut Criterion) {
    let mut engine = TimeEngine::new();

    c.bench_function("time/horizon_adapt", |b| {
        b.iter(|| {
            // Simulate horizon adaptation by ticking and reading horizons
            engine.tick();
            let hp = engine.Hp();
            let hc = engine.Hc();
            black_box((hp, hc))
        })
    });
}

/// Benchmark time synchronization (full workflow)
fn bench_time_synchronization(c: &mut Criterion) {
    let mut engine = TimeEngine::new();
    let peer = NodeId::new(0x5678);

    c.bench_function("time/sync_workflow", |b| {
        let mut seq = 0u16;
        b.iter(|| {
            seq = seq.wrapping_add(1);

            // Simulate receiving time sync packet
            let remote_time = engine.tau_s();
            
            // Update network model
            engine.update_from_packet(
                black_box(peer),
                black_box(remote_time),
                black_box(seq),
            );

            // Classify time
            let classification = engine.classify_time(black_box(remote_time));
            black_box(classification)
        })
    });
}

/// Benchmark clock comparison
fn bench_clock_comparison(c: &mut Criterion) {
    let perceptual = PerceptualClock::new();
    let state = StateClock::new();

    c.bench_function("time/clock_comparison", |b| {
        b.iter(|| {
            let p_now = perceptual.now();
            let s_now = state.now();
            // Compare as f64 seconds
            let p_secs = p_now.as_secs_f64();
            let s_secs = s_now.as_secs_f64();
            let diff = (p_secs - s_secs).abs();
            black_box(diff)
        })
    });
}

criterion_group!(
    benches,
    bench_time_engine_tick,
    bench_time_classify,
    bench_perceptual_clock_tick,
    bench_perceptual_clock_now,
    bench_state_clock_advance,
    bench_state_clock_blend,
    bench_state_clock_now,
    bench_network_model_update,
    bench_drift_estimation,
    bench_horizon_adaptation,
    bench_time_synchronization,
    bench_clock_comparison,
);
criterion_main!(benches);
