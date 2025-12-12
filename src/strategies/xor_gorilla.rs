use crate::TSPackedSamples;
use crate::TSSamples;

use crate::TSPackAttributes;
use crate::TSPackStrategyType;
use crate::TimeSeriesDataPacker;

fn f64_to_bits(v: f64) -> u64 {
    v.to_bits()
}

fn bits_to_f64(b: u64) -> f64 {
    f64::from_bits(b)
}

pub fn xor_pack(samples: &[TSSamples]) -> Vec<TSPackedSamples> {
    if samples.is_empty() {
        return Vec::new();
    }

    let mut result: Vec<TSPackedSamples> = Vec::new();

    let mut prev_bits = f64_to_bits(samples[0].1);
    result.push(((samples[0].0, samples[0].0), samples[0].1));

    for &(ts, val) in &samples[1..] {
        let bits = f64_to_bits(val);
        let xor = prev_bits ^ bits;

        let xor_as_f64 = bits_to_f64(xor);
        result.push(((ts, ts), xor_as_f64));
        prev_bits = bits;
    }

    result
}

pub fn xor_unpack(packed: &[TSPackedSamples]) -> Vec<TSSamples> {
    if packed.is_empty() {
        return Vec::new();
    }

    let mut result: Vec<TSSamples> = Vec::new();

    let first_val = packed[0].1;
    result.push((packed[0].0 .0, first_val));

    let mut prev_bits = f64_to_bits(first_val);

    for &((ts, _), xor_as_f64) in &packed[1..] {
        let xor_bits = f64_to_bits(xor_as_f64);
        let new_bits = prev_bits ^ xor_bits;
        let new_val = bits_to_f64(new_bits);
        result.push((ts, new_val));
        prev_bits = new_bits;
    }

    result
}

#[cfg(test)]
mod xor_gorilla_tests {
    use super::*;

    #[test]
    fn test_xor_pack_unpack_roundtrip() {
        let samples = vec![(0.0, 100.0), (0.1, 101.0), (0.2, 105.5), (0.3, -50.25)];

        let packed = xor_pack(&samples);
        let unpacked = xor_unpack(&packed);

        assert_eq!(samples.len(), unpacked.len());
        for (orig, rec) in samples.iter().zip(unpacked.iter()) {
            assert!((orig.0 - rec.0).abs() < 1e-12);
            assert!((orig.1 - rec.1).abs() < 1e-12);
        }
    }

    #[test]
    fn test_xor_strategy_integration() {
        let samples = vec![(0.0, 10.0), (0.1, 20.0), (0.2, 30.0)];

        let mut packer = TimeSeriesDataPacker::new();
        let attrs = TSPackAttributes {
            strategy_types: vec![TSPackStrategyType::TSPackXorStrategy],
            microseconds_time_window: 1_000_000,
        };

        let packed = packer.pack(samples.clone(), attrs).unwrap();
        let unpacked = xor_unpack(&packed);

        assert_eq!(samples.len(), unpacked.len());
        for (orig, rec) in samples.iter().zip(unpacked.iter()) {
            assert!((orig.1 - rec.1).abs() < 1e-12);
        }
    }

    #[test]
    fn test_xor_empty() {
        let samples: Vec<TSSamples> = vec![];
        let packed = xor_pack(&samples);
        let unpacked = xor_unpack(&packed);
        assert!(packed.is_empty());
        assert!(unpacked.is_empty());
    }
}
