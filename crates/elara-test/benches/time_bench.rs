//! Benchmarks for ELARA time engine operations

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use elara_core::NodeId;
use elara_time::{NetworkModel, PerceptualClock, StateClock, TimeEngine};

fn bench_time_engine_tick(c: &mut Criterion) {
    let mut engine = TimeEngine::new();

    c.bench_function("time_engine_tick", |b| {
        b.iter(|| {
            engine.tick();
            black_box(engine.tau_s())
        })
    });
}

fn bench_perceptual_clock_tick(c: &mut Criterion) {
    let mut clock = PerceptualClock::new();

    c.bench_function("perceptual_clock_tick", |b| {
        b.iter(|| {
            clock.tick();
            black_box(clock.now())
        })
    });
}

fn bench_state_clock_advance(c: &mut Criterion) {
    let mut clock = StateClock::new();
    let dt = std::time::Duration::from_millis(10);

    c.bench_function("state_clock_advance", |b| {
        b.iter(|| {
            clock.advance(black_box(dt));
            black_box(clock.now())
        })
    });
}

fn bench_state_clock_blend(c: &mut Criterion) {
    let mut clock = StateClock::new();
    let dt = std::time::Duration::from_millis(10);

    c.bench_function("state_clock_blend", |b| {
        b.iter(|| {
            // Advance and blend correction
            clock.advance(black_box(dt));
            black_box(clock.now())
        })
    });
}

fn bench_network_model_update(c: &mut Criterion) {
    let mut model = NetworkModel::new();
    let peer = NodeId::new(0x1234);

    c.bench_function("network_model_update", |b| {
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

fn bench_time_classify(c: &mut Criterion) {
    let engine = TimeEngine::new();
    let base = engine.tau_s();

    c.bench_function("time_classify", |b| {
        b.iter(|| engine.classify_time(black_box(base)))
    });
}

criterion_group!(
    benches,
    bench_time_engine_tick,
    bench_perceptual_clock_tick,
    bench_state_clock_advance,
    bench_state_clock_blend,
    bench_network_model_update,
    bench_time_classify,
);
criterion_main!(benches);
