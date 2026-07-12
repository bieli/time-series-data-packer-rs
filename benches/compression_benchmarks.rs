use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use time_series_data_packer_rs::*;

const WINDOW_US: u64 = 1_000;

fn make_constant_samples(size: usize) -> Vec<TSSamples> {
    (0..size).map(|i| (i as f64 * 0.001, 100.0)).collect()
}

fn make_incremental_samples(size: usize) -> Vec<TSSamples> {
    (0..size)
        .map(|i| (i as f64 * 0.001, 100.0 + i as f64 * 0.001))
        .collect()
}

fn pack_with_strategy(samples: &[TSSamples], strategy: TSPackStrategyType) -> Vec<TSPackedSamples> {
    let attrs = TSPackAttributes {
        strategy_types: vec![strategy],
        microseconds_time_window: WINDOW_US,
        precision_epsilon: TSPackPrecisionDataType::IoTSensors.epsilon(),
    };
    let mut packer = TimeSeriesDataPacker::new();
    packer.pack(samples.to_vec(), attrs).unwrap()
}

fn benchmark_pack_strategies(c: &mut Criterion) {
    let sizes = [100, 1_000, 10_000, 100_000];

    for size in sizes {
        let samples = make_constant_samples(size);
        let mut group = c.benchmark_group(format!("pack_constant_{size}"));
        group.throughput(Throughput::Elements(size as u64));

        group.bench_function("similar_values", |b| {
            b.iter(|| {
                black_box(pack_with_strategy(
                    black_box(&samples),
                    TSPackStrategyType::TSPackSimilarValuesStrategy,
                ))
            })
        });

        group.bench_function("mean_5pct", |b| {
            b.iter(|| {
                black_box(pack_with_strategy(
                    black_box(&samples),
                    TSPackStrategyType::TSPackMeanStrategy {
                        values_compression_percent: 5,
                    },
                ))
            })
        });

        group.bench_function("delta", |b| {
            b.iter(|| {
                black_box(pack_with_strategy(
                    black_box(&samples),
                    TSPackStrategyType::TSPackDeltaStrategy,
                ))
            })
        });

        group.bench_function("xor_gorilla", |b| {
            b.iter(|| {
                black_box(pack_with_strategy(
                    black_box(&samples),
                    TSPackStrategyType::TSPackXorStrategy,
                ))
            })
        });

        group.finish();
    }
}

fn benchmark_xor_gorilla_incremental(c: &mut Criterion) {
    let sizes = [1_000, 10_000, 100_000];

    for size in sizes {
        let samples = make_incremental_samples(size);
        let mut group = c.benchmark_group(format!("xor_gorilla_incremental_{size}"));
        group.throughput(Throughput::Elements(size as u64));

        group.bench_function("pack", |b| {
            b.iter(|| {
                black_box(pack_with_strategy(
                    black_box(&samples),
                    TSPackStrategyType::TSPackXorStrategy,
                ))
            })
        });

        let packed = pack_with_strategy(&samples, TSPackStrategyType::TSPackXorStrategy);

        group.bench_function("unpack", |b| {
            b.iter(|| black_box(TSPackXorGorillaStrategy::unpack(black_box(&packed))))
        });

        group.finish();
    }
}

criterion_group!(benches, benchmark_pack_strategies, benchmark_xor_gorilla_incremental);
criterion_main!(benches);
