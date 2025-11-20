# time-series-data-packer-rs
Time series data packer written in Rust language

## Motivations
In a lot of my IoT projects, I have a pressure on storage size after time series data collection with miliseconds precission.
Yes, I'm using OPen Sourece greate data warehouses and inside engines do a lot of work for me, but one day I decided to find better - my own - way to time series data compressions.
This is an experimental project with saving storage size for time series data connected to specific domains (not random data for sure).

## API definitions
- structs:
  - `TSPackAttributes`
    - values_compression_percent - if we have in time series values i.e. 100, 100, 102, 98, 100, 99, to pack those sieries of values to 100, we need to set this parameter to `5`
  - `TSSamples`
    - f64 - timestamp in seconds
    - f64 - real value
  - `TSPackedSamples`
    - (f64, f64) - timestamps ranges in seconds
    - f64 - mean value (based on `values_compression_percent` parameter setting for `pack` function)
  
- `TimeSeriesDataPacker` - object with methods:
  - `fn pack(samples: Vec<TSSamples>, attr: TSPackAttributes) -> Vec<TSPackedSamples>` - packer functon
  - `fn unpack( -> Vec<TSPackedSamples>) -> Vec<TSSamples>` - unpacker functon


## TODO list
- [ ] CI
- [ ] crates package distribution
- [ ] prepare great real tests examples from various group of IoT sensors
- [ ] compare random sequences compressions rates with real data from IoT sensors
- [ ] add Python interface based on PyO3 library
- [ ] public Python package in TEST and official python packages for inc. popularity
- [ ] measure resources (RAM, CPU, IO) required to pack and unpack data with diffirent time ranges
- [ ] think about packed data buckets concept by time domain: minutes, hours, daily, weekly, monthly (kind of specific packed measurements partitions, becouse in time-series data analytics in IoT there are a very specific situations, when you need to select all historical data, usually you points of interests are selected by known short time ranges)
- [ ] think about lossless data packing algo.
