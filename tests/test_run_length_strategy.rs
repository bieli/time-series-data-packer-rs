use time_series_data_packer_rs::{
    TSPackAttributes, TSPackPrecisionDataType, TSPackRunLengthStrategy, TSPackStrategyType,
    TSSamples, TimeSeriesDataPacker,
};

#[test]
fn test_run_length_strategy_packs_constant_runs() {
    let samples: Vec<TSSamples> = vec![
        (0.0, 100.0),
        (0.1, 100.0),
        (0.2, 100.0),
        (0.3, 101.0),
        (0.4, 101.0),
        (0.5, 100.0),
    ];

    let mut packer = TimeSeriesDataPacker::new();
    let attrs = TSPackAttributes {
        strategy_types: vec![TSPackStrategyType::TSPackRunLengthStrategy],
        microseconds_time_window: 1_000_000,
        precision_epsilon: TSPackPrecisionDataType::IoTSensors.epsilon(),
    };

    let packed = packer.pack(samples, attrs).unwrap();

    assert_eq!(packed.len(), 3);
    assert_eq!(packed[0], ((0.0, 0.2), 100.0));
    assert_eq!(packed[1], ((0.3, 0.4), 101.0));
    assert_eq!(packed[2], ((0.5, 0.5), 100.0));
}

#[test]
fn test_run_length_packer_unpack_matches_strategy_unpack() {
    let samples: Vec<TSSamples> = vec![(0.0, 1.0), (0.1, 1.0), (0.2, 2.0), (0.3, 2.0), (0.4, 2.0)];

    let mut packer = TimeSeriesDataPacker::new();
    let attrs = TSPackAttributes {
        strategy_types: vec![TSPackStrategyType::TSPackRunLengthStrategy],
        microseconds_time_window: 1_000_000,
        precision_epsilon: 0.0,
    };

    let packed = packer.pack(samples, attrs).unwrap();
    let (_attrs, from_packer) = packer.unpack();
    let from_strategy = TSPackRunLengthStrategy::unpack(&packed);

    assert_eq!(from_packer, from_strategy);
}
