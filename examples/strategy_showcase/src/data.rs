use std::fs::File;
use std::path::{Path, PathBuf};

use csv::ReaderBuilder;
use time_series_data_packer_rs::TSSamples;

/// Load a CSV from `examples/data/<filename>`.
///
/// Supports headers `timestamp_us,value` (microseconds) or `time,value` (seconds).
pub fn load_csv(filename: &str) -> Result<Vec<TSSamples>, Box<dyn std::error::Error>> {
    let path = resolve_data_path(filename);
    let file = File::open(&path)?;

    let mut reader = ReaderBuilder::new().has_headers(true).from_reader(file);
    let headers = reader.headers()?.clone();

    let ts_is_seconds = headers
        .iter()
        .any(|h| h.eq_ignore_ascii_case("time") && !h.contains("us"));

    let mut samples = Vec::new();

    for record in reader.records() {
        let record = record?;
        let ts: f64 = record[0].parse()?;
        let value: f64 = record[1].parse()?;

        let ts_seconds = if ts_is_seconds { ts } else { ts / 1_000_000.0 };
        samples.push((ts_seconds, value));
    }

    Ok(samples)
}

fn resolve_data_path(filename: &str) -> PathBuf {
    let candidates = [
        PathBuf::from("examples/data").join(filename),
        PathBuf::from("../data").join(filename),
        PathBuf::from("../../examples/data").join(filename),
    ];

    for path in &candidates {
        if path.exists() {
            return path.clone();
        }
    }

    Path::new("examples/data").join(filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_iot_temperature_sample() {
        let samples = load_csv("iot_temperature_sensor.csv").expect("csv");
        assert!(samples.len() > 40);
        assert!((samples[0].1 - 26.5).abs() < 1e-9);
    }
}
