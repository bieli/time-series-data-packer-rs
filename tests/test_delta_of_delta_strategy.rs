use time_series_data_packer_rs::{
    TSPackAttributes, TSPackDeltaOfDeltaStrategy, TSPackStrategyType, TSSamples,
    TimeSeriesDataPacker,
};

#[test]
fn test_delta_of_delta_strategy_lossless_roundtrip() {
    let samples: Vec<TSSamples> = vec![
        (0.0, 10.0),
        (0.1, 12.0),
        (0.2, 15.0),
        (0.3, 19.0),
        (0.4, 24.0),
    ];

    let mut packer = TimeSeriesDataPacker::new();
    let attrs = TSPackAttributes {
        strategy_types: vec![TSPackStrategyType::TSPackDeltaOfDeltaStrategy],
        microseconds_time_window: 1_000_000,
        precision_epsilon: 0.1,
    };

    let packed = packer.pack(samples.clone(), attrs).unwrap();
    let recovered = TSPackDeltaOfDeltaStrategy::unpack(&packed);

    assert_eq!(samples.len(), recovered.len());
    for (orig, rec) in samples.iter().zip(recovered.iter()) {
        assert!((orig.0 - rec.0).abs() < 1e-12);
        assert!((orig.1 - rec.1).abs() < 1e-12);
    }
}

#[test]
fn test_delta_of_delta_packer_unpack_returns_encoded_values() {
    let samples: Vec<TSSamples> = vec![(0.0, 10.0), (0.1, 12.0), (0.2, 11.0)];

    let expected: Vec<TSSamples> = vec![(0.0, 10.0), (0.1, 2.0), (0.2, -3.0)];

    let mut packer = TimeSeriesDataPacker::new();
    let attrs = TSPackAttributes {
        strategy_types: vec![TSPackStrategyType::TSPackDeltaOfDeltaStrategy],
        microseconds_time_window: 1_000_000,
        precision_epsilon: 0.0,
    };

    packer.pack(samples, attrs).unwrap();
    let (_attrs, encoded) = packer.unpack();

    assert_eq!(expected, encoded);
}
