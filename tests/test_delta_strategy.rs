use time_series_data_packer_rs::{
    TSPackAttributes, TSPackStrategyType, TSSamples, TimeSeriesDataPacker,
};

#[test]
fn test_delta_strategy_roundtrip() {
    let mut packer = TimeSeriesDataPacker::new();

    let samples: Vec<TSSamples> = vec![
        (0.0, 1.0),
        (0.1, 1.2),
        (0.2, 0.9),
        (0.3, 1.5)];

    let expected: Vec<TSSamples> = vec![
        (0.0, 1.0),
        (0.1, 0.19999999999999996),
        (0.2, -0.29999999999999993),
        (0.3, 0.6),
    ];

    let attrs = TSPackAttributes {
        strategy_types: vec![TSPackStrategyType::TSPackDeltaStrategy],
        microseconds_time_window: 1_000_000,
        precision_epsilon: 0.01,
    };

    let packed = packer.pack(samples.clone(), attrs.clone()).unwrap();
    let (_attrs_back, unpacked) = packer.unpack();

    assert_eq!(samples.len(), unpacked.len());
    assert_eq!(expected, unpacked);
}
