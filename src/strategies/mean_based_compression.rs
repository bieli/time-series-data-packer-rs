use crate::helpers::merge_adjacent_equal_value_ranges;
use crate::TSPackedSamples;
use crate::TSSamples;

pub fn mean_pack(samples: &[TSSamples], percent: u8) -> Vec<TSPackedSamples> {
    if samples.is_empty() {
        return Vec::new();
    }

    let avg = {
        let sum: f64 = samples.iter().map(|(_, v)| v).sum();
        sum / (samples.len() as f64)
    };

    let tol = (percent as f64 / 100.0) * avg;
    let lower = avg - tol;
    let upper = avg + tol;

    let mut result: Vec<TSPackedSamples> = Vec::new();

    let mut group_start_ts: Option<f64> = None;
    let mut group_end_ts: Option<f64> = None;

    for &(ts, v) in samples {
        if v >= lower && v <= upper {
            if group_start_ts.is_none() {
                group_start_ts = Some(ts);
                group_end_ts = Some(ts);
            } else {
                group_end_ts = Some(ts);
            }
        } else {
            if let (Some(gs), Some(ge)) = (group_start_ts, group_end_ts) {
                result.push(((gs, ge), avg));
                group_start_ts = None;
                group_end_ts = None;
            }
            result.push(((ts, ts), v));
        }
    }

    if let (Some(gs), Some(ge)) = (group_start_ts, group_end_ts) {
        result.push(((gs, ge), avg));
    }

    merge_adjacent_equal_value_ranges(result)
}

pub fn mean_refine_packs(packs: Vec<TSPackedSamples>, percent: u8) -> Vec<TSPackedSamples> {
    if packs.is_empty() {
        return packs;
    }

    let mut merged: Vec<TSPackedSamples> = Vec::new();

    let mut current = packs[0];

    for &next in &packs[1..] {
        let avg = current.1;
        let tol = (percent as f64 / 100.0) * avg;
        let lower = avg - tol;
        let upper = avg + tol;

        if next.1 >= lower && next.1 <= upper {
            current = ((current.0 .0, next.0 .1), avg);
        } else {
            merged.push(current);
            current = next;
        }
    }

    merged.push(current);
    merged
}
