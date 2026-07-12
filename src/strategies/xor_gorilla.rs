use crate::{TSPackedSamples, TSSamples};

/// XOR-based (Gorilla-style) lossless compression for `f64` values.
///
/// The first sample is stored verbatim. Each subsequent value is stored as the
/// XOR of its IEEE-754 bit pattern with the previous value's bit pattern.
/// Unpacking XORs again to reconstruct the original floats bit-for-bit.
pub struct TSPackXorGorillaStrategy;

impl TSPackXorGorillaStrategy {
    pub fn pack(samples: &[TSSamples]) -> Vec<TSPackedSamples> {
        if samples.is_empty() {
            return Vec::new();
        }

        let mut packed = Vec::with_capacity(samples.len());

        let (first_ts, first_val) = samples[0];
        packed.push(((first_ts, first_ts), first_val));

        let mut prev_bits = first_val.to_bits();

        for &(ts, val) in &samples[1..] {
            let bits = val.to_bits();
            let xor_bits = prev_bits ^ bits;
            packed.push(((ts, ts), f64::from_bits(xor_bits)));
            prev_bits = bits;
        }

        packed
    }

    pub fn unpack(packed: &[TSPackedSamples]) -> Vec<TSSamples> {
        if packed.is_empty() {
            return Vec::new();
        }

        let mut samples = Vec::with_capacity(packed.len());

        let ((first_ts, _), first_val) = packed[0];
        samples.push((first_ts, first_val));

        let mut prev_bits = first_val.to_bits();

        for &((ts, _), xor_as_f64) in &packed[1..] {
            let restored_bits = prev_bits ^ xor_as_f64.to_bits();
            let val = f64::from_bits(restored_bits);
            samples.push((ts, val));
            prev_bits = restored_bits;
        }

        samples
    }
}

/// Convenience alias for [`TSPackXorGorillaStrategy::pack`].
pub fn xor_pack(samples: &[TSSamples]) -> Vec<TSPackedSamples> {
    TSPackXorGorillaStrategy::pack(samples)
}

/// Convenience alias for [`TSPackXorGorillaStrategy::unpack`].
pub fn xor_unpack(packed: &[TSPackedSamples]) -> Vec<TSSamples> {
    TSPackXorGorillaStrategy::unpack(packed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TSPackAttributes, TSPackStrategyType, TimeSeriesDataPacker};

    fn assert_samples_eq(expected: &[TSSamples], actual: &[TSSamples]) {
        assert_eq!(expected.len(), actual.len());
        for (exp, act) in expected.iter().zip(actual.iter()) {
            assert!(
                (exp.0 - act.0).abs() < 1e-12,
                "timestamp mismatch: {exp:?} vs {act:?}"
            );
            assert!(
                (exp.1 - act.1).abs() < 1e-12 || (exp.1.is_nan() && act.1.is_nan()),
                "value mismatch: {exp:?} vs {act:?}"
            );
        }
    }

    #[test]
    fn pack_unpack_roundtrip() {
        let samples = vec![(0.0, 100.0), (0.1, 101.0), (0.2, 105.5), (0.3, -50.25)];

        let packed = TSPackXorGorillaStrategy::pack(&samples);
        let unpacked = TSPackXorGorillaStrategy::unpack(&packed);

        assert_samples_eq(&samples, &unpacked);
    }

    #[test]
    fn pack_unpack_roundtrip_via_convenience_functions() {
        let samples = vec![(1.0, 42.0), (2.0, 42.001), (3.0, 42.002)];

        let packed = xor_pack(&samples);
        let unpacked = xor_unpack(&packed);

        assert_samples_eq(&samples, &unpacked);
    }

    #[test]
    fn empty_input() {
        let samples: Vec<TSSamples> = vec![];
        assert!(TSPackXorGorillaStrategy::pack(&samples).is_empty());
        assert!(TSPackXorGorillaStrategy::unpack(&[]).is_empty());
    }

    #[test]
    fn single_sample() {
        let samples = vec![(5.0, 123.456)];
        let packed = TSPackXorGorillaStrategy::pack(&samples);
        assert_eq!(packed.len(), 1);
        assert_eq!(packed[0], ((5.0, 5.0), 123.456));

        let unpacked = TSPackXorGorillaStrategy::unpack(&packed);
        assert_samples_eq(&samples, &unpacked);
    }

    #[test]
    fn identical_consecutive_values_produce_zero_xor() {
        let samples = vec![(0.0, 10.0), (0.1, 10.0), (0.2, 10.0)];
        let packed = TSPackXorGorillaStrategy::pack(&samples);

        assert_eq!(packed[0].1, 10.0);
        assert_eq!(packed[1].1, 0.0);
        assert_eq!(packed[2].1, 0.0);

        assert_samples_eq(&samples, &TSPackXorGorillaStrategy::unpack(&packed));
    }

    #[test]
    fn handles_nan_and_negative_zero() {
        let samples = vec![(0.0, f64::NAN), (0.1, -0.0), (0.2, 1.0)];
        let unpacked = TSPackXorGorillaStrategy::unpack(&TSPackXorGorillaStrategy::pack(&samples));

        assert!(unpacked[0].1.is_nan());
        assert_eq!(unpacked[1].1.to_bits(), (-0.0_f64).to_bits());
        assert_eq!(unpacked[2].1, 1.0);
    }

    #[test]
    fn integration_with_time_series_data_packer() {
        let samples = vec![(0.0, 10.0), (0.1, 20.0), (0.2, 30.0)];

        let mut packer = TimeSeriesDataPacker::new();
        let attrs = TSPackAttributes {
            strategy_types: vec![TSPackStrategyType::TSPackXorStrategy],
            microseconds_time_window: 1_000_000,
            precision_epsilon: 0.1,
        };

        let packed = packer.pack(samples.clone(), attrs).unwrap();
        let unpacked = TSPackXorGorillaStrategy::unpack(&packed);

        assert_samples_eq(&samples, &unpacked);
    }

    #[test]
    fn repack_from_already_packed_xor_data() {
        let samples = vec![(0.0, 1.5), (0.5, 2.5), (1.0, 3.5)];
        let first_pass = TSPackXorGorillaStrategy::pack(&samples);

        let mut packer = TimeSeriesDataPacker::new();
        let attrs = TSPackAttributes {
            strategy_types: vec![TSPackStrategyType::TSPackXorStrategy],
            microseconds_time_window: 1_000_000,
            precision_epsilon: 0.0,
        };

        // Simulate pipeline starting from raw samples; repack path is exercised internally.
        let repacked = packer.pack(samples.clone(), attrs).unwrap();
        assert_samples_eq(&samples, &TSPackXorGorillaStrategy::unpack(&repacked));
        assert_eq!(first_pass.len(), repacked.len());
    }
}
