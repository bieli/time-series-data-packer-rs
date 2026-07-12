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

### Type aliases
- `TSSamples` - `(f64, f64)` - timestamp in seconds, value
- `TSPackedSamples` - `((f64, f64), f64)` - timestamp range `(start, end)` in seconds, packed value

### Enums

#### `TSPackStrategyType`
Available compression strategies (can be chained in `TSPackAttributes::strategy_types`):

| Variant | Description |
|---------|-------------|
| `TSPackSimilarValuesStrategy` | Groups consecutive samples with equal values (within `precision_epsilon`) into a single time range. Default strategy for repetitive sensor readings. |
| `TSPackMeanStrategy { values_compression_percent: u8 }` | Groups values within +/-N% of the window mean. E.g. `5` packs `100, 102, 98, 100, 99` around their average. |
| `TSPackXorStrategy` | **XOR Gorilla** - lossless bit-level compression inspired by Facebook Gorilla TSDB. First value stored raw; each subsequent value stored as XOR of IEEE-754 bit patterns with the previous value. Use [`TSPackXorGorillaStrategy::unpack`] for lossless recovery. |
| `TSPackDeltaStrategy` | Stores first value raw, then successive deltas (`value - previous`). Lossless for arithmetic differences. |
| `TSPackRunLengthStrategy` | **Run-length encoding (RLE)** - collapses consecutive identical values (exact IEEE-754 bit match) into a single time range. Run length is implicit in `(start_ts, end_ts)`. |

#### `TSPackPrecisionDataType`
Preset precision profiles with an `epsilon()` helper:

| Variant | Epsilon |
|---------|---------|
| `MilisValues` | `1e-3` |
| `WavDerivedAudio` | `1e-4` |
| `IoTSensors` | `1e-5` |
| `HighPrecisionTelemetry` | `1e-7` |
| `ScientificData` | `1e-9` |

#### `TSPackError`
- `InvalidWindow` - returned when `microseconds_time_window` is `0`

### Structs

#### `TSPackAttributes`
Configuration passed to [`TimeSeriesDataPacker::pack`]:

| Field | Type | Description |
|-------|------|-------------|
| `strategy_types` | `Vec<TSPackStrategyType>` | Compression strategies applied in order per time window |
| `microseconds_time_window` | `u64` | Window size in microseconds; samples are split before packing |
| `precision_epsilon` | `f64` | Tolerance for value comparison and rounding (ignored for bit-exact strategies: XOR Gorilla, Delta) |

#### `TimeSeriesDataPacker`
Main packer state object.

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `fn new() -> Self` | Create an empty packer |
| `pack` | `fn pack(&mut self, samples: Vec<TSSamples>, attributes: TSPackAttributes) -> Result<Vec<TSPackedSamples>, TSPackError>` | Sort, window, apply strategies, and store packed output |
| `unpack` | `fn unpack(&self) -> (Option<TSPackAttributes>, Vec<TSSamples>)` | Expand packed ranges to timestamp/value pairs (returns encoded values for XOR/Delta strategies, not reconstructed originals) |

### Strategy modules (direct use)

#### `TSPackXorGorillaStrategy`
XOR Gorilla lossless float compression.

| Method | Signature | Description |
|--------|-----------|-------------|
| `pack` | `fn pack(samples: &[TSSamples]) -> Vec<TSPackedSamples>` | Encode samples with XOR bit-pattern compression |
| `unpack` | `fn unpack(packed: &[TSPackedSamples]) -> Vec<TSSamples>` | Decode XOR-compressed data back to original `f64` values bit-for-bit |

Convenience functions: `xor_pack`, `xor_unpack` (aliases for the above).

#### `TSPackDeltaStrategy`
Delta encoding for float series.

| Method | Signature | Description |
|--------|-----------|-------------|
| `pack` | `fn pack(samples: &[TSSamples]) -> Vec<TSPackedSamples>` | Store first value raw, then deltas |
| `unpack` | `fn unpack(packed: &[TSPackedSamples]) -> Vec<TSSamples>` | Reconstruct original values from deltas |

#### `TSPackRunLengthStrategy`
Run-length encoding for repeated values.

| Method | Signature | Description |
|--------|-----------|-------------|
| `pack` | `fn pack(samples: &[TSSamples]) -> Vec<TSPackedSamples>` | Collapse consecutive identical values into time ranges |
| `unpack` | `fn unpack(packed: &[TSPackedSamples]) -> Vec<TSSamples>` | Expand each run to start and end timestamp/value pairs |

Convenience functions: `rle_pack`, `rle_unpack` (aliases for the above).

### Run-length encoding - how it works

**Packing:**
1. Scan consecutive samples with the same value (compared by IEEE-754 bit pattern).
2. Store one entry per run: `((start_ts, end_ts), value)`.

**Unpacking:**
1. Each run expands to its start and end points (intermediate timestamps within a run are not reconstructed).

**Example:**
```rust
use time_series_data_packer_rs::*;

let samples = vec![
    (0.0, 100.0), (0.1, 100.0), (0.2, 100.0),
    (0.3, 101.0), (0.4, 101.0),
];

let packed = TSPackRunLengthStrategy::pack(&samples);
// [((0.0, 0.2), 100.0), ((0.3, 0.4), 101.0)]

let expanded = TSPackRunLengthStrategy::unpack(&packed);
// [(0.0, 100.0), (0.2, 100.0), (0.3, 101.0), (0.4, 101.0)]
```

### XOR Gorilla - how it works

**Packing:**
1. First value is stored as-is.
2. Each subsequent value is XOR'd with the previous value's IEEE-754 bit pattern.
3. The XOR result is stored as `f64` (lossless bit reinterpretation).

**Unpacking:**
1. First value is read raw.
2. Each subsequent XOR delta is applied to reconstruct the original bits.

**Lossless recovery example:**
```rust
use time_series_data_packer_rs::*;

let samples = vec![(0.0, 100.0), (0.1, 101.0), (0.2, 105.5)];

let mut packer = TimeSeriesDataPacker::new();
let attrs = TSPackAttributes {
    strategy_types: vec![TSPackStrategyType::TSPackXorStrategy],
    microseconds_time_window: 1_000_000,
    precision_epsilon: TSPackPrecisionDataType::IoTSensors.epsilon(),
};

let packed = packer.pack(samples.clone(), attrs)?;
let recovered = TSPackXorGorillaStrategy::unpack(&packed);
assert_eq!(samples, recovered);
```

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
        precision_epsilon: TSPackPrecisionDataType::IoTSensors.epsilon(),
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

## Performance benchmarks

Run Criterion benchmarks for all strategies:

```bash
cargo bench --bench compression_benchmarks
```

Benchmark groups:
- `pack_constant_{size}` - packing constant-value series with Similar Values, Mean, Delta, XOR Gorilla, and Run-length strategies
- `xor_gorilla_incremental_{size}` - XOR Gorilla pack and unpack on slowly changing values
- `run_length_alternating_{size}` - Run-length pack and unpack on alternating-value series

## TODO list
- [X] CI
- [x] crates package distribution
- [ ] I want to have visual UI, to create signals patterns and use it as input to easy wrinting unit tests
- [ ] prepare great real tests examples from various group of IoT sensors
- [ ] compare random sequences compressions rates with real data from IoT sensors
- [ ] add Python interface based on PyO3 library
- [ ] public Python package in TEST and official python packages for inc. popularity
- [ ] measure resources (RAM, CPU, IO) required to pack and unpack data with diffirent time ranges
- [ ] think about packed data buckets concept by time domain: minutes, hours, daily, weekly, monthly (kind of specific packed measurements partitions, becouse in time-series data analytics in IoT there are a very specific situations, when you need to select all historical data, usually you points of interests are selected by known short time ranges)
- [ ] think about lossless data packing algo.
