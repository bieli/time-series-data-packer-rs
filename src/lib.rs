use thiserror::Error;

// A single sample: (timestamp_seconds, value)
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

        todo!();

        Ok(vec!())
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
