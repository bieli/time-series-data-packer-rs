pub mod helpers;
pub mod strategies;

use std::cmp::Ordering;
use thiserror::Error;

use crate::helpers::apply_strategy;
use crate::helpers::finalize_to_packed;
use crate::helpers::merge_adjacent_equal_value_ranges;
use crate::helpers::split_into_windows;
use crate::helpers::Representation;

// A single raw sample: (timestamp_seconds, value)
pub type TSSamples = (f64, f64);

// A single packed sample: ((start_seconds, end_seconds), value)
pub type TSPackedSamples = ((f64, f64), f64);

#[derive(Debug, Clone)]
pub enum TSPackStrategyType {
    TSPackSimilarValuesStrategy,
    TSPackMeanStrategy { values_compression_percent: u8 },
    TSPackXorStrategy,
}

#[derive(Debug, Clone)]
pub struct TSPackAttributes {
    pub strategy_types: Vec<TSPackStrategyType>,
    pub microseconds_time_window: u64,
}

#[derive(Debug, Error, PartialEq)]
pub enum TSPackError {
    #[error("microseconds_time_window must be > 0")]
    InvalidWindow,
}

#[derive(Debug, Clone)]
pub struct TimeSeriesDataPacker {
    attributes: Option<TSPackAttributes>,
    original_samples: Vec<TSSamples>,
    packed_samples: Vec<TSPackedSamples>,
}

impl Default for TimeSeriesDataPacker {
    fn default() -> Self {
        Self {
            attributes: None,
            original_samples: Vec::new(),
            packed_samples: Vec::new(),
        }
    }
}

impl TimeSeriesDataPacker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pack(
        &mut self,
        mut samples: Vec<TSSamples>,
        attributes: TSPackAttributes,
    ) -> Result<Vec<TSPackedSamples>, TSPackError> {
        if attributes.microseconds_time_window == 0 {
            return Err(TSPackError::InvalidWindow);
        }

        samples.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));

        let windows = split_into_windows(&samples, attributes.microseconds_time_window);

        let mut packed_all: Vec<TSPackedSamples> = Vec::new();

        for window_samples in windows {
            let mut current_representation = Representation::Raw(window_samples);

            for strategy in &attributes.strategy_types {
                current_representation = apply_strategy(current_representation, strategy);
            }

            let packed = finalize_to_packed(current_representation);
            packed_all.extend(packed);
        }

        let merged = merge_adjacent_equal_value_ranges(packed_all);

        self.attributes = Some(attributes.clone());
        self.original_samples = samples;
        self.packed_samples = merged.clone();

        Ok(merged)
    }

    pub fn unpack(&self) -> (Option<TSPackAttributes>, Vec<TSSamples>) {
        let mut result: Vec<TSSamples> = Vec::new();

        for &((start, end), value) in &self.packed_samples {
            result.push((start, value));

            if end != start {
                result.push((end, value));
            }
        }

        (self.attributes.clone(), result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_similar_values_strategy_check_pack_and_unpack() {
        let samples = vec![
            (0.0, 100.0),
            (0.1, 100.0),
            (0.2, 100.0),
            (0.3, 101.0),
            (0.4, 101.0),
            (0.5, 100.0),
        ];

        let mut packer = TimeSeriesDataPacker::new();
        let attrs = TSPackAttributes {
            strategy_types: vec![TSPackStrategyType::TSPackSimilarValuesStrategy],
            microseconds_time_window: 1_000_000, // 1 second windows
        };

        let packed = packer.pack(samples.clone(), attrs).unwrap();
        assert_eq!(packed.len(), 3);
        assert_eq!(packed[0], ((0.0, 0.2), 100.0));
        assert_eq!(packed[1], ((0.3, 0.4), 101.0));
        assert_eq!(packed[2], ((0.5, 0.5), 100.0));

        let (_attrs, unpacked) = packer.unpack();
        assert_eq!(unpacked.len(), 5);
        assert_eq!(unpacked[0], (0.0, 100.0));

        // Here you can discovery limitation of the current unpack
        // logic with TSPackSimilarValuesStrategy - to have lossless
        // we need to extends struct TSPackedSamples to contains timestamps

        // NOT RECOVERED ITEM: assert_eq!(unpacked[1], (0.1, 100.0));

        assert_eq!(unpacked[1], (0.2, 100.0));
        assert_eq!(unpacked[2], (0.3, 101.0));
        assert_eq!(unpacked[3], (0.4, 101.0));
        assert_eq!(unpacked[4], (0.5, 100.0));
    }

    #[test]
    fn test_similar_values_strategy_on_real_measurements_example() {
        let samples = vec![
            (1.431142, 26.5),
            (1.513428, 26.5),
            (1.650571, 26.5),
            (1.979714, 26.5),
            (2.253999, 26.5),
            (2.583142, 26.5),
            (2.802571, 26.5),
            (3.021999, 26.5),
            (3.131714, 26.5),
            (3.323714, 26.5),
            (3.488285, 26.5),
            (3.625428, 26.5),
            (3.735142, 26.5),
            (3.872285, 26.5),
            (3.981999, 26.5),
            (4.119142, 26.5),
            (4.256285, 26.5),
            (4.338571, 26.5),
            (4.448285, 26.5),
            (4.530571, 26.5),
            (4.612857, 26.5),
            (4.695142, 26.5),
            (4.777428, 26.5),
            (4.832285, 26.5),
            (4.859714, 26.8),
            (4.887142, 26.8),
            (4.941999, 26.8),
            (4.996857, 26.8),
            (5.024285, 26.8),
            (5.079142, 27.1),
            (5.106571, 27.1),
            (5.133999, 27.1),
            (5.188857, 27.1),
            (5.216285, 27.1),
            (5.243714, 27.1),
            (5.271142, 27.1),
            (5.325999, 27.1),
            (5.353428, 27.1),
            (5.408285, 27.1),
            (5.435714, 27.1),
            (5.490571, 27.1),
            (5.517999, 27.1),
            (5.600285, 27.1),
            (5.709999, 27.1),
            (5.792285, 27.4),
            (5.819714, 27.8),
            (5.874571, 27.4),
            (5.956857, 27.4),
            (6.093999, 27.4),
            (6.231142, 27.4),
            (6.368285, 27.4),
            (6.477999, 27.8),
            (6.532857, 27.8),
            (6.642571, 28.4),
            (6.724857, 28.7),
        ];

        let mut packer = TimeSeriesDataPacker::new();
        let attrs = TSPackAttributes {
            strategy_types: vec![TSPackStrategyType::TSPackSimilarValuesStrategy],
            microseconds_time_window: 1_000_000, // 1 second windows
        };

        let packed = packer.pack(samples.clone(), attrs).unwrap();

        assert_eq!(packed.len(), 9);
        assert_eq!(packed[0], ((1.431142, 4.832285), 26.5));
        assert_eq!(packed[1], ((4.859714, 5.024285), 26.8));
        assert_eq!(packed[2], ((5.079142, 5.709999), 27.1));
        assert_eq!(packed[3], ((5.792285, 5.792285), 27.4));
        assert_eq!(packed[4], ((5.819714, 5.819714), 27.8));
        assert_eq!(packed[5], ((5.874571, 6.368285), 27.4));
        assert_eq!(packed[6], ((6.477999, 6.532857), 27.8));
        assert_eq!(packed[7], ((6.642571, 6.642571), 28.4));
        assert_eq!(packed[8], ((6.724857, 6.724857), 28.7));
    }

    #[test]
    fn test_mean_strategy_all_within_5_percents_tolerance() {
        // Values around 100 within +/- 5% tolerance (95..105)
        let samples = vec![
            (0.0, 100.0),
            (0.05, 100.0),
            (0.1, 102.0),
            (0.15, 98.0),
            (0.2, 100.0),
            (0.25, 99.0),
        ];

        let mut packer = TimeSeriesDataPacker::new();
        let attrs = TSPackAttributes {
            strategy_types: vec![TSPackStrategyType::TSPackMeanStrategy {
                values_compression_percent: 5,
            }],
            microseconds_time_window: 1_000_000, // 1 second windows
        };

        let packed = packer.pack(samples.clone(), attrs).unwrap();

        // Single range around average ~99.833(3)
        assert_eq!(packed.len(), 1);
        let ((start, end), val) = packed[0];
        assert!((start - 0.0).abs() < 1e-9);
        assert!((end - 0.25).abs() < 1e-9);
        assert!((val - 99.8333333).abs() < 1e-6);
    }

    #[test]
    fn test_mean_strategy_all_within_5_percents_tolerance_on_real_measurements_example() {
        let samples = vec![
            (1.431142, 26.5),
            (1.513428, 26.5),
            (1.650571, 26.5),
            (1.979714, 26.5),
            (2.253999, 26.5),
            (2.583142, 26.5),
            (2.802571, 26.5),
            (3.021999, 26.5),
            (3.131714, 26.5),
            (3.323714, 26.5),
            (3.488285, 26.5),
            (3.625428, 26.5),
            (3.735142, 26.5),
            (3.872285, 26.5),
            (3.981999, 26.5),
            (4.119142, 26.5),
            (4.256285, 26.5),
            (4.338571, 26.5),
            (4.448285, 26.5),
            (4.530571, 26.5),
            (4.612857, 26.5),
            (4.695142, 26.5),
            (4.777428, 26.5),
            (4.832285, 26.5),
            (4.859714, 26.8),
            (4.887142, 26.8),
            (4.941999, 26.8),
            (4.996857, 26.8),
            (5.024285, 26.8),
            (5.079142, 27.1),
            (5.106571, 27.1),
            (5.133999, 27.1),
            (5.188857, 27.1),
            (5.216285, 27.1),
            (5.243714, 27.1),
            (5.271142, 27.1),
            (5.325999, 27.1),
            (5.353428, 27.1),
            (5.408285, 27.1),
            (5.435714, 27.1),
            (5.490571, 27.1),
            (5.517999, 27.1),
            (5.600285, 27.1),
            (5.709999, 27.1),
            (5.792285, 27.4),
            (5.819714, 27.8),
            (5.874571, 27.4),
            (5.956857, 27.4),
            (6.093999, 27.4),
            (6.231142, 27.4),
            (6.368285, 27.4),
            (6.477999, 27.8),
            (6.532857, 27.8),
            (6.642571, 28.4),
            (6.724857, 28.7),
        ];

        let mut packer = TimeSeriesDataPacker::new();
        let attrs = TSPackAttributes {
            strategy_types: vec![TSPackStrategyType::TSPackMeanStrategy {
                values_compression_percent: 5,
            }],
            microseconds_time_window: 1_000_000, // 1 second windows
        };

        let packed = packer.pack(samples.clone(), attrs).unwrap();

        assert_eq!(packed.len(), 4);
        assert_eq!(packed[0], ((1.431142, 4.612857), 26.5));
        assert_eq!(packed[1], ((4.695142, 5.600285), 26.950000000000014));
        assert_eq!(packed[2], ((5.709999, 6.642571), 27.572727272727274));
        assert_eq!(packed[3], ((6.724857, 6.724857), 28.7));
    }

    #[test]
    fn test_invalid_window_error_in_data_packer_pack() {
        let samples = vec![];

        let mut packer = TimeSeriesDataPacker::new();
        let attrs = TSPackAttributes {
            strategy_types: vec![TSPackStrategyType::TSPackSimilarValuesStrategy],
            microseconds_time_window: 0,
        };

        let result = packer.pack(samples.clone(), attrs);

        assert_eq!(result, Err(TSPackError::InvalidWindow));
    }
}
