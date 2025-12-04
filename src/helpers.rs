use std::cmp::Ordering;

use crate::TSPackStrategyType;
use crate::TSPackedSamples;
use crate::TSSamples;

use crate::strategies::similar_values::similar_values_pack;

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
) -> Representation {
    match strategy {
        TSPackStrategyType::TSPackSimilarValuesStrategy => match representation {
            Representation::Raw(samples) => Representation::Packed(similar_values_pack(&samples)),
            Representation::Packed(packs) => todo!(),
        },
        &TSPackStrategyType::TSPackMeanStrategy { .. } => todo!(),
    }
}

pub fn finalize_to_packed(representation: Representation) -> Vec<TSPackedSamples> {
    match representation {
        Representation::Raw(samples) => samples.iter().map(|(ts, v)| ((*ts, *ts), *v)).collect(),
        Representation::Packed(packs) => packs,
    }
}

pub fn approx_touching(end: f64, start: f64) -> bool {
    const EPS: f64 = 1e-12;
    (end - start).abs() <= EPS || end <= start
}

pub fn merge_adjacent_equal_value_ranges(mut packs: Vec<TSPackedSamples>) -> Vec<TSPackedSamples> {
    if packs.is_empty() {
        return packs;
    }

    packs.sort_by(|a, b| a.0 .0.partial_cmp(&b.0 .0).unwrap_or(Ordering::Equal));

    let mut merged: Vec<TSPackedSamples> = Vec::new();
    let mut current = packs[0];

    for &next in &packs[1..] {
        if current.1 == next.1 && approx_touching(current.0 .1, next.0 .0) {
            current = ((current.0 .0, next.0 .1), current.1);
        } else {
            merged.push(current);
            current = next;
        }
    }

    merged.push(current);
    merged
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
}
