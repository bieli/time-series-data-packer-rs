use crate::{TSPackedSamples, TSSamples};

/// Simple-8b variable-bit integer packing applied to scaled value deltas.
///
/// Floating-point deltas are converted to integers using `scale = 1 / precision_epsilon`,
/// zigzag-encoded, batched into 64-bit Simple-8b words, and stored as `f64` via
/// bit reinterpretation. The first sample is stored as an anchor entry.
pub struct TSPackSimple8bStrategy;

/// Simple-8b encoding modes: `(values_per_word, bits_per_value)`.
const SIMPLE8B_MODES: &[(usize, u8)] = &[
    (240, 0),
    (60, 1),
    (30, 2),
    (20, 3),
    (15, 4),
    (12, 5),
    (10, 6),
    (8, 7),
    (7, 8),
    (6, 10),
    (5, 12),
    (4, 15),
    (3, 20),
    (2, 30),
    (1, 60),
];

/// Tag stored in `start_ts` for value Simple-8b word entries.
pub const SIMPLE8B_VALUE_WORD_TAG: f64 = f64::NEG_INFINITY;

/// Tag stored in `start_ts` for timestamp delta Simple-8b word entries.
pub const SIMPLE8B_TIME_WORD_TAG: f64 = f64::INFINITY;

#[inline]
pub fn scale_from_epsilon(precision_epsilon: f64) -> f64 {
    if precision_epsilon > 0.0 {
        1.0 / precision_epsilon
    } else {
        1_000.0
    }
}

#[inline]
fn zigzag_encode(value: i64) -> u64 {
    ((value << 1) ^ (value >> 63)) as u64
}

#[inline]
fn zigzag_decode(value: u64) -> i64 {
    ((value >> 1) as i64) ^ (-((value & 1) as i64))
}

pub fn simple8b_encode(values: &[u64]) -> Vec<u64> {
    if values.is_empty() {
        return Vec::new();
    }

    let mut words = Vec::new();
    let mut index = 0;

    while index < values.len() {
        let mut selected: Option<(usize, usize)> = None;

        for (mode, &(count, bits)) in SIMPLE8B_MODES.iter().enumerate() {
            if index + count > values.len() {
                continue;
            }

            let chunk = &values[index..index + count];
            let max_val = chunk.iter().copied().max().unwrap_or(0);

            if bits == 0 && max_val == 0 {
                selected = Some((mode, count));
                break;
            }

            if bits > 0 && max_val < (1u64 << bits) {
                selected = Some((mode, count));
                break;
            }
        }

        let (mode, count) = selected.unwrap_or((14, 1));
        let bits = SIMPLE8B_MODES[mode].1;

        let mut word = (mode as u64) << 60;
        let mut shift = 0;

        for offset in 0..count {
            let value = values[index + offset];
            word |= value << shift;
            shift += bits;
        }

        words.push(word);
        index += count;
    }

    words
}

pub fn simple8b_decode(words: &[u64]) -> Vec<u64> {
    let mut values = Vec::new();

    for &word in words {
        let mode = (word >> 60) as usize;
        let (count, bits) = SIMPLE8B_MODES[mode];
        let mut shift = 0;

        for _ in 0..count {
            if bits == 0 {
                values.push(0);
            } else {
                let mask = (1u64 << bits) - 1;
                values.push((word >> shift) & mask);
                shift += bits;
            }
        }
    }

    values
}

impl TSPackSimple8bStrategy {
    pub fn pack(samples: &[TSSamples], precision_epsilon: f64) -> Vec<TSPackedSamples> {
        if samples.is_empty() {
            return Vec::new();
        }

        let scale = scale_from_epsilon(precision_epsilon);
        let (first_ts, first_value) = samples[0];
        let last_ts = samples.last().map(|(ts, _)| *ts).unwrap_or(first_ts);

        let mut packed = Vec::new();
        packed.push(((first_ts, last_ts), first_value));

        if samples.len() == 1 {
            return packed;
        }

        let mut value_deltas = Vec::with_capacity(samples.len() - 1);
        let mut time_deltas_us = Vec::with_capacity(samples.len() - 1);
        let mut prev_value = first_value;
        let mut prev_ts = first_ts;

        for &(ts, value) in &samples[1..] {
            let scaled_delta = ((value - prev_value) * scale).round() as i64;
            value_deltas.push(zigzag_encode(scaled_delta));

            let delta_us = ((ts - prev_ts) * 1_000_000.0).round().max(0.0) as u64;
            time_deltas_us.push(delta_us);

            prev_value = value;
            prev_ts = ts;
        }

        for word in simple8b_encode(&value_deltas) {
            packed.push(((SIMPLE8B_VALUE_WORD_TAG, 0.0), f64::from_bits(word)));
        }

        for word in simple8b_encode(&time_deltas_us) {
            packed.push(((SIMPLE8B_TIME_WORD_TAG, 0.0), f64::from_bits(word)));
        }

        packed
    }

    pub fn unpack(packed: &[TSPackedSamples], precision_epsilon: f64) -> Vec<TSSamples> {
        if packed.is_empty() {
            return Vec::new();
        }

        let scale = scale_from_epsilon(precision_epsilon);
        let ((first_ts, _), first_value) = packed[0];

        let value_words: Vec<u64> = packed
            .iter()
            .filter(|((tag, _), _)| *tag == SIMPLE8B_VALUE_WORD_TAG)
            .map(|(_, value)| value.to_bits())
            .collect();

        let time_words: Vec<u64> = packed
            .iter()
            .filter(|((tag, _), _)| *tag == SIMPLE8B_TIME_WORD_TAG)
            .map(|(_, value)| value.to_bits())
            .collect();

        let value_deltas = simple8b_decode(&value_words);
        let time_deltas_us = simple8b_decode(&time_words);

        let mut samples = Vec::with_capacity(value_deltas.len() + 1);
        samples.push((first_ts, first_value));

        let mut current_value = first_value;
        let mut current_ts = first_ts;

        for index in 0..value_deltas.len() {
            let delta = zigzag_decode(value_deltas[index]) as f64 / scale;
            current_value += delta;

            if index < time_deltas_us.len() {
                current_ts += time_deltas_us[index] as f64 / 1_000_000.0;
            } else {
                current_ts += 1.0;
            }

            samples.push((current_ts, current_value));
        }

        samples
    }
}

/// Convenience alias for [`TSPackSimple8bStrategy::pack`].
pub fn simple8b_pack(samples: &[TSSamples], precision_epsilon: f64) -> Vec<TSPackedSamples> {
    TSPackSimple8bStrategy::pack(samples, precision_epsilon)
}

/// Convenience alias for [`TSPackSimple8bStrategy::unpack`].
pub fn simple8b_unpack(packed: &[TSPackedSamples], precision_epsilon: f64) -> Vec<TSSamples> {
    TSPackSimple8bStrategy::unpack(packed, precision_epsilon)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        TSPackAttributes, TSPackPrecisionDataType, TSPackStrategyType, TimeSeriesDataPacker,
    };

    fn assert_values_close(expected: &[TSSamples], actual: &[TSSamples], tolerance: f64) {
        assert_eq!(expected.len(), actual.len());
        for (exp, act) in expected.iter().zip(actual.iter()) {
            assert!(
                (exp.0 - act.0).abs() < 1e-6,
                "timestamp mismatch: {exp:?} vs {act:?}"
            );
            assert!(
                (exp.1 - act.1).abs() < tolerance,
                "value mismatch: {exp:?} vs {act:?}"
            );
        }
    }

    #[test]
    fn encode_decode_roundtrip() {
        let values = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let words = simple8b_encode(&values);
        let decoded = simple8b_decode(&words);
        assert_eq!(values, decoded);
    }

    #[test]
    fn encode_decode_zeros() {
        let values = vec![0; 240];
        let decoded = simple8b_decode(&simple8b_encode(&values));
        assert_eq!(values, decoded);
    }

    #[test]
    fn pack_unpack_roundtrip() {
        let samples = vec![(0.0, 100.0), (1.0, 100.5), (2.0, 101.0), (3.0, 102.25)];
        let epsilon = TSPackPrecisionDataType::MilisValues.epsilon();

        let packed = TSPackSimple8bStrategy::pack(&samples, epsilon);
        let unpacked = TSPackSimple8bStrategy::unpack(&packed, epsilon);

        assert_values_close(&samples, &unpacked, 1e-3);
    }

    #[test]
    fn pack_unpack_with_negative_deltas() {
        let samples = vec![(0.0, 10.0), (1.0, 9.5), (2.0, 9.0), (3.0, 10.5)];
        let epsilon = TSPackPrecisionDataType::MilisValues.epsilon();

        let packed = TSPackSimple8bStrategy::pack(&samples, epsilon);
        let unpacked = TSPackSimple8bStrategy::unpack(&packed, epsilon);

        assert_values_close(&samples, &unpacked, 1e-3);
    }

    #[test]
    fn empty_and_single_sample() {
        assert!(TSPackSimple8bStrategy::pack(&[], 1e-3).is_empty());
        assert!(TSPackSimple8bStrategy::unpack(&[], 1e-3).is_empty());

        let single = vec![(5.0, 42.0)];
        let packed = TSPackSimple8bStrategy::pack(&single, 1e-3);
        assert_eq!(packed, vec![((5.0, 5.0), 42.0)]);

        let unpacked = TSPackSimple8bStrategy::unpack(&packed, 1e-3);
        assert_eq!(unpacked, single);
    }

    #[test]
    fn anchor_entry_preserves_first_value() {
        let samples = vec![(0.0, 100.0), (1.0, 100.5), (2.0, 101.0)];
        let epsilon = 1e-3;
        let packed = TSPackSimple8bStrategy::pack(&samples, epsilon);

        assert_eq!(packed[0].1, 100.0);
        assert!(packed.len() > 1);
    }

    #[test]
    fn integration_with_time_series_data_packer() {
        let samples = vec![(0.0, 100.0), (1.0, 100.5), (2.0, 101.0), (3.0, 102.25)];

        let mut packer = TimeSeriesDataPacker::new();
        let attrs = TSPackAttributes {
            strategy_types: vec![TSPackStrategyType::TSPackSimple8bStrategy],
            microseconds_time_window: 10_000_000,
            precision_epsilon: TSPackPrecisionDataType::MilisValues.epsilon(),
        };

        let packed = packer.pack(samples.clone(), attrs.clone()).unwrap();
        let recovered = TSPackSimple8bStrategy::unpack(&packed, attrs.precision_epsilon);

        assert_values_close(&samples, &recovered, 1e-3);
    }

    #[test]
    fn repack_from_already_packed_data() {
        let samples = vec![(0.0, 1.0), (1.0, 1.2), (2.0, 1.4)];
        let epsilon = 1e-3;
        let first_pass = TSPackSimple8bStrategy::pack(&samples, epsilon);

        let mut packer = TimeSeriesDataPacker::new();
        let attrs = TSPackAttributes {
            strategy_types: vec![TSPackStrategyType::TSPackSimple8bStrategy],
            microseconds_time_window: 10_000_000,
            precision_epsilon: epsilon,
        };

        let repacked = packer.pack(samples.clone(), attrs).unwrap();
        assert_eq!(first_pass.len(), repacked.len());
        assert_values_close(
            &samples,
            &TSPackSimple8bStrategy::unpack(&repacked, epsilon),
            1e-3,
        );
    }
}
