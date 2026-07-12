use crate::{TSPackedSamples, TSSamples};

/// Delta-of-delta encoding for `f64` value series.
///
/// Stores the first value raw, the second as the first delta, and each subsequent
/// entry as the delta-of-delta (change in delta). Unpacking reconstructs values
/// exactly via double-precision arithmetic.
pub struct TSPackDeltaOfDeltaStrategy;

impl TSPackDeltaOfDeltaStrategy {
    pub fn pack(samples: &[TSSamples]) -> Vec<TSPackedSamples> {
        if samples.is_empty() {
            return Vec::new();
        }

        let mut packed = Vec::with_capacity(samples.len());

        let (t0, v0) = samples[0];
        packed.push(((t0, t0), v0));

        if samples.len() == 1 {
            return packed;
        }

        let v1 = samples[0].1;
        let v2 = samples[1].1;
        let first_delta = v2 - v1;
        packed.push(((samples[1].0, samples[1].0), first_delta));

        if samples.len() == 2 {
            return packed;
        }

        let mut prev_delta = first_delta;

        for i in 2..samples.len() {
            let vi = samples[i].1;
            let vim1 = samples[i - 1].1;
            let delta = vi - vim1;
            let delta_of_delta = delta - prev_delta;
            packed.push(((samples[i].0, samples[i].0), delta_of_delta));
            prev_delta = delta;
        }

        packed
    }

    pub fn unpack(packed: &[TSPackedSamples]) -> Vec<TSSamples> {
        if packed.is_empty() {
            return Vec::new();
        }

        let mut samples = Vec::with_capacity(packed.len());

        let ((t0, _), v0) = packed[0];
        samples.push((t0, v0));

        if packed.len() == 1 {
            return samples;
        }

        let ((t1, _), first_delta) = packed[1];
        let v1 = v0 + first_delta;
        samples.push((t1, v1));

        if packed.len() == 2 {
            return samples;
        }

        let mut prev_delta = first_delta;
        let mut last_value = v1;

        for &((ts, _), delta_of_delta) in &packed[2..] {
            let delta = prev_delta + delta_of_delta;
            let value = last_value + delta;
            samples.push((ts, value));
            prev_delta = delta;
            last_value = value;
        }

        samples
    }
}

/// Convenience alias for [`TSPackDeltaOfDeltaStrategy::pack`].
pub fn delta_of_delta_pack(samples: &[TSSamples]) -> Vec<TSPackedSamples> {
    TSPackDeltaOfDeltaStrategy::pack(samples)
}

/// Convenience alias for [`TSPackDeltaOfDeltaStrategy::unpack`].
pub fn delta_of_delta_unpack(packed: &[TSPackedSamples]) -> Vec<TSSamples> {
    TSPackDeltaOfDeltaStrategy::unpack(packed)
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
                (exp.1 - act.1).abs() < 1e-12,
                "value mismatch: {exp:?} vs {act:?}"
            );
        }
    }

    #[test]
    fn pack_unpack_roundtrip_monotonic() {
        let samples = vec![
            (0.0, 10.0),
            (0.1, 12.0),
            (0.2, 15.0),
            (0.3, 19.0),
            (0.4, 24.0),
        ];

        let packed = TSPackDeltaOfDeltaStrategy::pack(&samples);
        let unpacked = TSPackDeltaOfDeltaStrategy::unpack(&packed);

        assert_eq!(packed[1].1, 2.0);
        assert_eq!(packed[2].1, 1.0);
        assert_eq!(packed[3].1, 1.0);
        assert_eq!(packed[4].1, 1.0);
        assert_samples_eq(&samples, &unpacked);
    }

    #[test]
    fn pack_unpack_roundtrip_varied() {
        let samples = vec![
            (0.0, 100.0),
            (0.05, 99.5),
            (0.10, 101.0),
            (0.15, 100.25),
            (0.20, 100.50),
        ];

        let unpacked =
            TSPackDeltaOfDeltaStrategy::unpack(&TSPackDeltaOfDeltaStrategy::pack(&samples));
        assert_samples_eq(&samples, &unpacked);
    }

    #[test]
    fn empty_single_and_pair() {
        assert!(TSPackDeltaOfDeltaStrategy::pack(&[]).is_empty());
        assert!(TSPackDeltaOfDeltaStrategy::unpack(&[]).is_empty());

        let single = vec![(0.0, 5.0)];
        assert_samples_eq(
            &single,
            &TSPackDeltaOfDeltaStrategy::unpack(&TSPackDeltaOfDeltaStrategy::pack(&single)),
        );

        let pair = vec![(0.0, 5.0), (0.1, 7.0)];
        assert_samples_eq(
            &pair,
            &TSPackDeltaOfDeltaStrategy::unpack(&TSPackDeltaOfDeltaStrategy::pack(&pair)),
        );
    }

    #[test]
    fn integration_with_time_series_data_packer() {
        let samples = vec![(0.0, 10.0), (0.1, 12.0), (0.2, 11.0), (0.3, 13.5)];

        let mut packer = TimeSeriesDataPacker::new();
        let attrs = TSPackAttributes {
            strategy_types: vec![TSPackStrategyType::TSPackDeltaOfDeltaStrategy],
            microseconds_time_window: 1_000_000,
            precision_epsilon: 0.1,
        };

        let packed = packer.pack(samples.clone(), attrs).unwrap();
        let recovered = TSPackDeltaOfDeltaStrategy::unpack(&packed);

        assert_samples_eq(&samples, &recovered);
    }

    #[test]
    fn repack_from_already_packed_data() {
        let samples = vec![(0.0, 1.0), (0.1, 1.5), (0.2, 2.1)];
        let first_pass = TSPackDeltaOfDeltaStrategy::pack(&samples);

        let mut packer = TimeSeriesDataPacker::new();
        let attrs = TSPackAttributes {
            strategy_types: vec![TSPackStrategyType::TSPackDeltaOfDeltaStrategy],
            microseconds_time_window: 1_000_000,
            precision_epsilon: 0.0,
        };

        let repacked = packer.pack(samples.clone(), attrs).unwrap();
        assert_eq!(first_pass.len(), repacked.len());
        assert_samples_eq(&samples, &TSPackDeltaOfDeltaStrategy::unpack(&repacked));
    }
}
