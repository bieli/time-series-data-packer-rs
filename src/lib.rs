pub mod helpers;
pub mod strategies;


use std::cmp::Ordering;
use thiserror::Error;


use crate::helpers::Representation;
use crate::helpers::apply_strategy;
use crate::helpers::finalize_to_packed;
use crate::helpers::split_into_windows;
use crate::strategies::similar_values::merge_adjacent_equal_value_ranges;


// A single raw sample: (timestamp_seconds, value)
pub type TSSamples = (f64, f64);

// A single packed sample: ((start_seconds, end_seconds), value)
pub type TSPackedSamples = ((f64, f64), f64);

#[derive(Debug, Clone)]
pub enum TSPackStrategyType {
    TSPackSimilarValuesStrategy,
    TSPackMeanStrategy { values_compression_percent: u8 },
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

        samples.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(Ordering::Equal)
        });

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

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_similar_values_strategy() {
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
    }

    #[test]
    fn test_invalid_window_error_in_data_packer_pack() {
        let samples = vec![
        ];

        let mut packer = TimeSeriesDataPacker::new();
        let attrs = TSPackAttributes {
            strategy_types: vec![TSPackStrategyType::TSPackSimilarValuesStrategy],
            microseconds_time_window: 0
        };

        let result = packer.pack(samples.clone(), attrs);

        assert_eq!(result, Err(TSPackError::InvalidWindow));
    }
}
