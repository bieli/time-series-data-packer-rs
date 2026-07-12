use time_series_data_packer_rs::{
    TSPackAttributes, TSPackStrategyType, TSPackXorGorillaStrategy, TSSamples, TimeSeriesDataPacker,
};

#[test]
fn test_xor_gorilla_strategy_lossless_roundtrip() {
    let samples: Vec<TSSamples> = vec![(0.0, 100.0), (0.1, 101.0), (0.2, 105.5), (0.3, -50.25)];

    let mut packer = TimeSeriesDataPacker::new();
    let attrs = TSPackAttributes {
        strategy_types: vec![TSPackStrategyType::TSPackXorStrategy],
        microseconds_time_window: 1_000_000,
        precision_epsilon: 0.1,
    };

    let packed = packer.pack(samples.clone(), attrs).unwrap();
    let recovered = TSPackXorGorillaStrategy::unpack(&packed);

    assert_eq!(samples.len(), recovered.len());
    for (orig, rec) in samples.iter().zip(recovered.iter()) {
        assert!((orig.0 - rec.0).abs() < 1e-12);
        assert!((orig.1 - rec.1).abs() < 1e-12);
    }
}

#[test]
fn test_xor_gorilla_packer_unpack_returns_encoded_values() {
    let samples: Vec<TSSamples> = vec![(0.0, 10.0), (0.1, 20.0), (0.2, 30.0)];

    let mut packer = TimeSeriesDataPacker::new();
    let attrs = TSPackAttributes {
        strategy_types: vec![TSPackStrategyType::TSPackXorStrategy],
        microseconds_time_window: 1_000_000,
        precision_epsilon: 0.0,
    };

    let packed = packer.pack(samples.clone(), attrs).unwrap();
    let (_attrs, encoded) = packer.unpack();

    assert_eq!(encoded.len(), samples.len());
    assert_eq!(encoded[0], (0.0, 10.0));
    assert_eq!(encoded[1].0, 0.1);
    assert_eq!(encoded[2].0, 0.2);
    assert_eq!(encoded[1].1, packed[1].1);
    assert_eq!(encoded[2].1, packed[2].1);
}
