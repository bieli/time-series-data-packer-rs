use std::cmp::Ordering;

use crate::TSPackStrategyType;
use crate::TSPackedSamples;
use crate::TSSamples;

use crate::strategies::similar_values::similar_values_pack;

use crate::strategies::mean_based_compression::mean_pack;
use crate::strategies::mean_based_compression::mean_refine_packs;

use crate::strategies::delta::TSPackDeltaStrategy;
use crate::strategies::run_length::TSPackRunLengthStrategy;
use crate::strategies::xor_gorilla::TSPackXorGorillaStrategy;

#[derive(Debug, Clone)]
pub enum Representation {
    // Raw samples: (ts, value)
    Raw(Vec<TSSamples>),
    // Packed ranges: ((start_ts, end_ts), value)
    Packed(Vec<TSPackedSamples>),
}

pub fn split_into_windows(samples: &[TSSamples], micro_window: u64) -> Vec<Vec<TSSamples>> {
    if samples.is_empty() {
        return Vec::new();
    }

    let mut windows: Vec<Vec<TSSamples>> = Vec::new();
    let mut current: Vec<TSSamples> = Vec::new();

    let mut window_start_ts = samples[0].0;
    let window_len_seconds = (micro_window as f64) / 1_000_000.0;

    for &(ts, val) in samples {
        if ts - window_start_ts <= window_len_seconds {
            current.push((ts, val));
        } else {
            if !current.is_empty() {
                windows.push(current.clone());
                current.clear();
            }
            window_start_ts = ts;
            current.push((ts, val));
        }
    }

    if !current.is_empty() {
        windows.push(current);
    }

    windows
}

pub fn apply_strategy(
    representation: Representation,
    strategy: &TSPackStrategyType,
    precision_epsilon: f64,
) -> Representation {
    match strategy {
        TSPackStrategyType::TSPackSimilarValuesStrategy => match representation {
            Representation::Raw(samples) => {
                Representation::Packed(similar_values_pack(&samples, precision_epsilon))
            }
            Representation::Packed(packs) => {
                let merged = merge_adjacent_equal_value_ranges(packs, precision_epsilon);
                Representation::Packed(merged)
            }
        },
        TSPackStrategyType::TSPackMeanStrategy {
            values_compression_percent,
        } => match representation {
            Representation::Raw(samples) => Representation::Packed(mean_pack(
                &samples,
                *values_compression_percent,
                precision_epsilon,
            )),
            Representation::Packed(packs) => Representation::Packed(mean_refine_packs(
                packs,
                *values_compression_percent,
                precision_epsilon,
            )),
        },
        TSPackStrategyType::TSPackXorStrategy => match representation {
            Representation::Raw(samples) => {
                Representation::Packed(TSPackXorGorillaStrategy::pack(&samples))
            }
            Representation::Packed(packs) => {
                let raw = TSPackXorGorillaStrategy::unpack(&packs);
                Representation::Packed(TSPackXorGorillaStrategy::pack(&raw))
            }
        },
        TSPackStrategyType::TSPackDeltaStrategy => match representation {
            Representation::Raw(samples) => {
                let packed = TSPackDeltaStrategy::pack(&samples);
                Representation::Packed(packed)
            }
            Representation::Packed(packs) => {
                // unpack -> repack (same pattern as XOR)
                let raw = TSPackDeltaStrategy::unpack(&packs);
                let repacked = TSPackDeltaStrategy::pack(&raw);
                Representation::Packed(repacked)
            }
        },
        TSPackStrategyType::TSPackRunLengthStrategy => match representation {
            Representation::Raw(samples) => {
                Representation::Packed(TSPackRunLengthStrategy::pack(&samples))
            }
            Representation::Packed(packs) => {
                let raw = TSPackRunLengthStrategy::unpack(&packs);
                Representation::Packed(TSPackRunLengthStrategy::pack(&raw))
            }
        },
    }
}

pub fn finalize_to_packed(rep: Representation, eps: f64) -> Vec<TSPackedSamples> {
    match rep {
        Representation::Raw(samples) => samples
            .into_iter()
            .map(|(t, v)| ((t, t), round_to_precision(v, eps)))
            .collect(),

        Representation::Packed(packs) => packs
            .into_iter()
            .map(|((s, e), v)| ((s, e), round_to_precision(v, eps)))
            .collect(),
    }
}

pub fn approx_touching(end: f64, start: f64) -> bool {
    const EPS: f64 = 1e-12;
    (end - start).abs() <= EPS || end <= start
}

pub fn approx_equal(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

pub fn merge_adjacent_equal_value_ranges(
    mut packed: Vec<TSPackedSamples>,
    eps: f64,
) -> Vec<TSPackedSamples> {
    if packed.is_empty() {
        return packed;
    }

    let mut result = Vec::new();
    let mut current = packed[0];

    for &next in &packed[1..] {
        let ((cur_start, cur_end), cur_val) = current;
        let ((next_start, next_end), next_val) = next;

        if approx_equal(cur_val, next_val, eps) {
            current = ((cur_start, next_end), cur_val);
        } else {
            result.push(current);
            current = next;
        }
    }

    result.push(current);
    result
}

#[inline]
pub fn uses_bit_exact_encoding(strategies: &[TSPackStrategyType]) -> bool {
    strategies.iter().any(|strategy| {
        matches!(
            strategy,
            TSPackStrategyType::TSPackXorStrategy | TSPackStrategyType::TSPackDeltaStrategy
        )
    })
}

pub fn round_to_precision(value: f64, eps: f64) -> f64 {
    if eps == 0.0 {
        return value;
    }

    let digits = (-eps.log10()).round() as i32;
    let factor = 10_f64.powi(digits);
    (value * factor).round() / factor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windowing_microseconds() {
        let samples = vec![
            (0.00, 1.0),
            (0.05, 1.0),
            (0.10, 2.0),
            (0.15, 2.0),
            (0.21, 3.0),
        ];

        let windows = split_into_windows(&samples, 100_000);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].len(), 3);
        assert_eq!(windows[1].len(), 2);
    }

    #[test]
    fn test_merge_with_epsilon() {
        let packed = vec![((0.0, 0.0), 0.0500000000001), ((1.0, 1.0), 0.0499999999999)];

        let merged = merge_adjacent_equal_value_ranges(packed, 1e-4);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0], ((0.0, 1.0), 0.0500000000001));
    }
}
