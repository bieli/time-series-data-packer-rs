use crate::TSPackAttributes;
use crate::TSPackStrategyType;
use crate::TSPackedSamples;
use crate::TSSamples;
use crate::TimeSeriesDataPacker;

#[inline]
fn approx_equal(a: f64, b: f64, eps: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_nan() || b.is_nan() {
        return false;
    }
    (a - b).abs() <= eps
}

pub fn similar_values_pack(samples: &[TSSamples], eps: f64) -> Vec<TSPackedSamples> {
    if samples.is_empty() {
        return Vec::new();
    }

    let mut result: Vec<TSPackedSamples> = Vec::new();

    let mut run_start_ts = samples[0].0;
    let mut prev_ts = samples[0].0;
    let mut current_value = samples[0].1;

    for &(ts, val) in &samples[1..] {
        if approx_equal(val, current_value, eps) {
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

#[cfg(test)]
mod similar_values_pack_tests {
    use super::*;

    #[test]
    fn test_similar_values_pack_with_3_digits_precision() {
        let samples: Vec<TSSamples> = vec![
            (35.0, 0.03),
            (59.0, 0.03),
            (71.0, 0.04),
            (83.0, 0.05),
            (95.0, 0.05),
            (107.0, 0.05),
            (119.0, 0.05),
            (130.0, 0.06),
            (142.0, 0.07),
            (166.0, 0.07),
            (178.0, 0.07),
            (214.0, 0.07),
            (226.0, 0.08),
            (250.0, 0.08),
            (261.0, 0.09),
        ];

        let expected = vec![
            ((35.0, 59.0), 0.03),
            ((71.0, 71.0), 0.04),
            ((83.0, 119.0), 0.05),
            ((130.0, 130.0), 0.06),
            ((142.0, 214.0), 0.07),
            ((226.0, 250.0), 0.08),
            ((261.0, 261.0), 0.09),
        ];

        let mut packer = TimeSeriesDataPacker::new();
        let attrs = TSPackAttributes {
            strategy_types: vec![TSPackStrategyType::TSPackSimilarValuesStrategy],
            microseconds_time_window: 1_000_000,
            precision_epsilon: 0.001,
        };

        let packed = packer.pack(samples.clone(), attrs.clone()).unwrap();

        assert_eq!(expected, packed);
    }

    #[test]
    fn test_similar_values_pack_with_2_digits_precision() {
        let samples: Vec<TSSamples> = vec![
            (35.0, 0.03),
            (59.0, 0.03),
            (71.0, 0.04),
            (83.0, 0.05),
            (95.0, 0.05),
            (107.0, 0.05),
            (119.0, 0.05),
            (130.0, 0.06),
            (142.0, 0.07),
            (166.0, 0.07),
            (178.0, 0.07),
            (214.0, 0.07),
            (226.0, 0.08),
            (250.0, 0.08),
            (261.0, 0.09),
        ];

        let expected = vec![
            ((35.0, 71.0), 0.03),
            ((83.0, 130.0), 0.05),
            ((142.0, 261.0), 0.07),
        ];

        let mut packer = TimeSeriesDataPacker::new();
        let attrs = TSPackAttributes {
            strategy_types: vec![TSPackStrategyType::TSPackSimilarValuesStrategy],
            microseconds_time_window: 1_000_000,
            precision_epsilon: 0.02,
        };

        let packed = packer.pack(samples.clone(), attrs.clone()).unwrap();

        assert_eq!(expected, packed);
    }
}
