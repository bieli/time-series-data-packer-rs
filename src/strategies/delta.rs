use crate::{TSPackedSamples, TSSamples};

pub struct TSPackDeltaStrategy;

impl TSPackDeltaStrategy {
    pub fn pack(samples: &[TSSamples]) -> Vec<TSPackedSamples> {
        if samples.is_empty() {
            return Vec::new();
        }

        let mut packed = Vec::with_capacity(samples.len());

        let (t0, v0) = samples[0];
        packed.push(((t0, t0), v0));

        let mut prev = v0;

        for &(t, v) in &samples[1..] {
            let delta = v - prev;
            packed.push(((t, t), delta));
            prev = v;
        }

        packed
    }

    pub fn unpack(packed: &[TSPackedSamples]) -> Vec<TSSamples> {
        if packed.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(packed.len());

        let ((t0, _), v0) = packed[0];
        result.push((t0, v0));

        let mut prev = v0;

        for &((t, _), delta) in &packed[1..] {
            let v = prev + delta;
            result.push((t, v));
            prev = v;
        }

        result
    }
}
