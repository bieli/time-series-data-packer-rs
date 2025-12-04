use std::cmp::Ordering;

use crate::TSSamples;
use crate::TSPackedSamples;

// Compress consecutive identical values into ranges.
// Each run becomes ((start_ts, end_ts), value).
pub fn similar_values_pack(samples: &[TSSamples]) -> Vec<TSPackedSamples> {
    if samples.is_empty() {
        return Vec::new();
    }

    let mut result: Vec<TSPackedSamples> = Vec::new();

    let mut run_start_ts = samples[0].0;
    let mut prev_ts = samples[0].0;
    let mut current_value = samples[0].1;

    for &(ts, val) in &samples[1..] {
        if val == current_value {
            prev_ts = ts;
        } else {
            result.push(((run_start_ts, prev_ts), current_value));
            run_start_ts = ts;
            prev_ts = ts;
            current_value = val;
        }
    }

    result.push(((run_start_ts, prev_ts), current_value));

    result
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

