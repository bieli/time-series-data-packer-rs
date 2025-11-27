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

#[derive(Debug, Error)]
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

