# time-series-data-packer-rs

![CI status](https://github.com/bieli/time-series-data-packer-rs/actions/workflows/test.yaml/badge.svg)
![github_tag](https://img.shields.io/github/v/tag/bieli/time-series-data-packer-rs)
[![Crates.io](https://img.shields.io/crates/v/time-series-data-packer-rs.svg)](https://crates.io/crates/time-series-data-packer-rs)
Time series data packer written in Rust language for data intensive IoT and IIoT projects.

## Motivations
In a lot of my IoT projects, I have a pressure on storage size after time series data collection with miliseconds precission.
Yes, I've been using Open Sourece great data warehouses, time-series dedicated databases and engines/databases do a lot of work for me, but one day I decided to find better - my own - way to time series data compressions.
This is an experimental project with saving storage size for time series data connected to specific domains (not random data for sure).

## API definitions
- structs:
  - `TSPackStrategyType`
    - `TSPackSimilarValuesStrategy` - similar values based simple methodology (repeatition of the same values will be packed - default strategy)
    - `TSPackMeanStrategy(values_compression_percent: u8)` - mean value based simple methodoloty (for first iteration). `values_compression_percent` parameter value explanations: if we have in time series values i.e. 100, 100, 102, 98, 100, 99, to pack those sieries of values to 100, we need to set this parameter to `5`. Means, we have avg. from series and we try to find, if values are in -5 to 5 range based on avg. value as a reference on values data window.
    - `TSPackXorStrategy`
      - Packing: First value stored raw. Each subsequent value is XOR’d with the previous value’s bit pattern. The XOR result is stored as an f64 (lossless since we reinterpret bits).
      - Unpacking: First value read raw. Each subsequent XOR is applied to reconstruct the original bits.
  - `TSPackAttributes`
    - `strategy_types: Vec<TSPackStrategyType>` - what method of compression we would like to use
    - `microseconds_time_window: u64` - time window to apply packing strategies in microseconds resolution (sometimes seconds means we have a 100k + similar measurements, so it's good to define more real limits for bare metal and sensor specific criteria for particular data domains)
  - `TSSamples`
    - f64 - timestamp in seconds
    - f64 - real value
  - `TSPackedSamples`
    - (f64, f64) - timestamps ranges in seconds
    - f64 - i.e using mean values strategy (based on `values_compression_percent` parameter setting for `pack` function - in case of used `TSPackMeanStrategy`)
  
- `TimeSeriesDataPacker` - object with methods:
  - `fn pack(samples: Vec<TSSamples>, attributes: TSPackAttributes) -> Vec<TSPackedSamples>` - packer functon
  - `fn unpack( -> Vec<TSPackedSamples>) -> (TSPackAttributes, Vec<TSSamples>)` - unpacker functon


### API usage example
```rust
use time_series_data_packer_rs::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let samples: Vec<TSSamples> = vec![
        (0.0, 100.0),
        (0.001, 100.0),
        (0.002, 102.0),
        (0.003, 98.0),
        (0.004, 100.0),
        (0.005, 99.0),
    ];

    let attrs = TSPackAttributes {
        strategy_types: vec![
            TSPackStrategyType::TSPackMeanStrategy { values_compression_percent: 5 },
        ],
        microseconds_time_window: 1_000, // 1 ms
    };

    let mut packer = TimeSeriesDataPacker::new();
    let packed = packer.pack(samples.clone(), attrs)?;
    println!("Packed: {:?}", packed);

    let (_attrs, original) = packer.unpack();
    println!("Original recovered: {:?}", original);
    
    println!("Samples == Original recovered: {:?}", samples == original);

    Ok(())
}
```
#### Results from API usage example after run
```bash
$ cargo run

Packed: [((0.0, 0.003), 100.0), ((0.004, 0.005), 99.5)]
Original recovered: [(0.0, 100.0), (0.001, 100.0), (0.002, 102.0), (0.003, 98.0), (0.004, 100.0), (0.005, 99.0)]
Samples == Original recovered: true
```


## TODO list
- [X] CI
- [ ] crates package distribution
- [ ] prepare great real tests examples from various group of IoT sensors
- [ ] compare random sequences compressions rates with real data from IoT sensors
- [ ] add Python interface based on PyO3 library
- [ ] public Python package in TEST and official python packages for inc. popularity
- [ ] measure resources (RAM, CPU, IO) required to pack and unpack data with diffirent time ranges
- [ ] think about packed data buckets concept by time domain: minutes, hours, daily, weekly, monthly (kind of specific packed measurements partitions, becouse in time-series data analytics in IoT there are a very specific situations, when you need to select all historical data, usually you points of interests are selected by known short time ranges)
- [ ] think about lossless data packing algo.
