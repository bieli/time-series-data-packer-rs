use crate::{TSPackedSamples, TSSamples};

/// Run-length encoding for consecutive identical `f64` values.
///
/// Consecutive samples with the same value (compared by IEEE-754 bit pattern)
/// are collapsed into a single `((start_ts, end_ts), value)` entry.
/// Run length is implicit in the timestamp range.
pub struct TSPackRunLengthStrategy;

#[inline]
fn values_equal(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits()
}

impl TSPackRunLengthStrategy {
    pub fn pack(samples: &[TSSamples]) -> Vec<TSPackedSamples> {
        if samples.is_empty() {
            return Vec::new();
        }

        let mut packed = Vec::new();

        let mut run_start_ts = samples[0].0;
        let mut prev_ts = samples[0].0;
        let mut current_value = samples[0].1;

        for &(ts, val) in &samples[1..] {
            if values_equal(val, current_value) {
                prev_ts = ts;
            } else {
                packed.push(((run_start_ts, prev_ts), current_value));
                run_start_ts = ts;
                prev_ts = ts;
                current_value = val;
            }
        }

        packed.push(((run_start_ts, prev_ts), current_value));
        packed
    }

    /// Expands each run to its start and end timestamp (same semantics as
    /// [`crate::TimeSeriesDataPacker::unpack`]).
    pub fn unpack(packed: &[TSPackedSamples]) -> Vec<TSSamples> {
        let mut samples = Vec::with_capacity(packed.len() * 2);

        for &((start, end), val) in packed {
            samples.push((start, val));
            if end != start {
                samples.push((end, val));
            }
        }

        samples
    }
}

/// Convenience alias for [`TSPackRunLengthStrategy::pack`].
pub fn rle_pack(samples: &[TSSamples]) -> Vec<TSPackedSamples> {
    TSPackRunLengthStrategy::pack(samples)
}

/// Convenience alias for [`TSPackRunLengthStrategy::unpack`].
pub fn rle_unpack(packed: &[TSPackedSamples]) -> Vec<TSSamples> {
    TSPackRunLengthStrategy::unpack(packed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        TSPackAttributes, TSPackPrecisionDataType, TSPackStrategyType, TimeSeriesDataPacker,
    };

    #[test]
    fn pack_unpack_roundtrip() {
        let samples = vec![
            (0.0, 100.0),
            (0.1, 100.0),
            (0.2, 100.0),
            (0.3, 101.0),
            (0.4, 101.0),
            (0.5, 100.0),
        ];

        let packed = TSPackRunLengthStrategy::pack(&samples);
        assert_eq!(packed.len(), 3);
        assert_eq!(packed[0], ((0.0, 0.2), 100.0));
        assert_eq!(packed[1], ((0.3, 0.4), 101.0));
        assert_eq!(packed[2], ((0.5, 0.5), 100.0));

        let unpacked = TSPackRunLengthStrategy::unpack(&packed);
        assert_eq!(
            unpacked,
            vec![
                (0.0, 100.0),
                (0.2, 100.0),
                (0.3, 101.0),
                (0.4, 101.0),
                (0.5, 100.0),
            ]
        );
    }

    #[test]
    fn empty_input() {
        assert!(TSPackRunLengthStrategy::pack(&[]).is_empty());
        assert!(TSPackRunLengthStrategy::unpack(&[]).is_empty());
    }

    #[test]
    fn single_sample() {
        let samples = vec![(0.0, 42.0)];
        let packed = TSPackRunLengthStrategy::pack(&samples);
        assert_eq!(packed, vec![((0.0, 0.0), 42.0)]);

        let unpacked = TSPackRunLengthStrategy::unpack(&packed);
        assert_eq!(unpacked, vec![(0.0, 42.0)]);
    }

    #[test]
    fn groups_nan_by_bit_pattern() {
        let samples = vec![(0.0, f64::NAN), (0.1, f64::NAN), (0.2, 1.0)];
        let packed = TSPackRunLengthStrategy::pack(&samples);

        assert_eq!(packed.len(), 2);
        assert_eq!(packed[0].0, (0.0, 0.1));
        assert!(packed[0].1.is_nan());
        assert_eq!(packed[1], ((0.2, 0.2), 1.0));
    }

    #[test]
    fn integration_with_time_series_data_packer() {
        let samples = vec![(0.0, 1.0), (0.1, 1.0), (0.2, 2.0), (0.3, 2.0), (0.4, 2.0)];

        let mut packer = TimeSeriesDataPacker::new();
        let attrs = TSPackAttributes {
            strategy_types: vec![TSPackStrategyType::TSPackRunLengthStrategy],
            microseconds_time_window: 1_000_000,
            precision_epsilon: TSPackPrecisionDataType::IoTSensors.epsilon(),
        };

        let packed = packer.pack(samples, attrs).unwrap();
        assert_eq!(packed.len(), 2);

        let unpacked = TSPackRunLengthStrategy::unpack(&packed);
        assert_eq!(unpacked[0], (0.0, 1.0));
        assert_eq!(unpacked[1], (0.1, 1.0));
        assert_eq!(unpacked[2], (0.2, 2.0));
        assert_eq!(unpacked[3], (0.4, 2.0));
    }

    #[test]
    fn repack_from_already_packed_data() {
        let samples = vec![(0.0, 5.0), (0.1, 5.0), (0.2, 6.0)];
        let first_pass = TSPackRunLengthStrategy::pack(&samples);

        let mut packer = TimeSeriesDataPacker::new();
        let attrs = TSPackAttributes {
            strategy_types: vec![TSPackStrategyType::TSPackRunLengthStrategy],
            microseconds_time_window: 1_000_000,
            precision_epsilon: 0.0,
        };

        let repacked = packer.pack(samples, attrs).unwrap();
        assert_eq!(first_pass, repacked);
    }
}
