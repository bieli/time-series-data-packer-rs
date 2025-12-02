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

