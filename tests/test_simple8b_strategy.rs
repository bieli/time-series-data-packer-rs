use time_series_data_packer_rs::{
    TSPackAttributes, TSPackPrecisionDataType, TSPackSimple8bStrategy, TSPackStrategyType,
    TSSamples, TimeSeriesDataPacker,
};

#[test]
fn test_simple8b_strategy_approximate_roundtrip() {
    let samples: Vec<TSSamples> = vec![(0.0, 100.0), (1.0, 100.5), (2.0, 101.0), (3.0, 102.25)];

    let mut packer = TimeSeriesDataPacker::new();
    let attrs = TSPackAttributes {
        strategy_types: vec![TSPackStrategyType::TSPackSimple8bStrategy],
        microseconds_time_window: 10_000_000,
        precision_epsilon: TSPackPrecisionDataType::MilisValues.epsilon(),
    };

    let packed = packer.pack(samples.clone(), attrs.clone()).unwrap();
    let recovered = TSPackSimple8bStrategy::unpack(&packed, attrs.precision_epsilon);

    assert_eq!(samples.len(), recovered.len());
    for (orig, rec) in samples.iter().zip(recovered.iter()) {
        assert!((orig.0 - rec.0).abs() < 1e-6);
        assert!((orig.1 - rec.1).abs() < 1e-3);
    }
}

#[test]
fn test_simple8b_packer_unpack_returns_anchor_and_words() {
    let samples: Vec<TSSamples> = vec![(0.0, 10.0), (1.0, 10.5), (2.0, 11.0)];

    let mut packer = TimeSeriesDataPacker::new();
    let attrs = TSPackAttributes {
        strategy_types: vec![TSPackStrategyType::TSPackSimple8bStrategy],
        microseconds_time_window: 10_000_000,
        precision_epsilon: TSPackPrecisionDataType::MilisValues.epsilon(),
    };

    let packed = packer.pack(samples, attrs).unwrap();
    let (_attrs, from_packer) = packer.unpack();

    assert_eq!(from_packer[0], (0.0, 10.0));
    assert!(packed.len() > 1);
    assert!(from_packer.len() >= packed.len());
}
