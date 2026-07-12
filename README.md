# time-series-data-packer-rs

![CI status](https://github.com/bieli/time-series-data-packer-rs/actions/workflows/test.yaml/badge.svg)
![github_tag](https://img.shields.io/github/v/tag/bieli/time-series-data-packer-rs)
[![Crates.io](https://img.shields.io/crates/v/time-series-data-packer-rs.svg)](https://crates.io/crates/time-series-data-packer-rs)

Time series data packer written in Rust language for data intensive IoT and IIoT projects.

## Motivations
In a lot of my IoT projects, I have a pressure on storage size after time series data collection with miliseconds precission.
Yes, I've been using Open Sourece great data warehouses, time-series dedicated databases and engines/databases do a lot of work for me, but one day I decided to find better - my own - way to time series data compressions.
This is an experimental project with saving storage size for time series data connected to specific domains (not random data for sure).

## Visual guide - strategies explained (ASCII)

This section is for new engineers. Every strategy transforms raw `(timestamp, value)` samples into packed `((start_ts, end_ts), value)` entries. Strategies can be **chained** inside a time window.

### Pipeline overview

```
  RAW SAMPLES                         PACKER PIPELINE                         PACKED OUTPUT
  -------------                       ---------------                         -------------

  (0.00, 100.0)  ──┐
  (0.01, 100.0)  ──┤  sort by time
  (0.02, 102.0)  ──┤  ──────────►  split into time windows  ──────────►  apply strategy chain
  (0.03,  98.0)  ──┤                  (microseconds)              (Similar → Mean → …)
  (0.04, 100.0)  ──┤                                                         │
  (0.05,  99.0)  ──┘                                                         ▼
                                                                    [((0.0, 0.25), 99.83)]
                                                                    fewer entries = smaller storage
```

**Data shapes:**

```
  TSSamples        = ( timestamp_sec , value )
  TSPackedSamples  = ( ( start_sec , end_sec ) , value )
```

**Strategy chain example:**

```
  Raw samples
       │
       ▼
  ┌─────────────────────┐
  │ Similar Values      │  merge values within epsilon
  └──────────┬──────────┘
             ▼
  ┌─────────────────────┐
  │ Run-length          │  collapse exact repeats
  └──────────┬──────────┘
             ▼
       Packed output
```

---

### 1. Similar Values (`TSPackSimilarValuesStrategy`)

Best for: sensor readings that stay flat with tiny noise.

```
  RAW (6 samples)                         PACKED (3 entries)
  time ─────────────────────────►         time ─────────────────────────►
        100   100   100  101  101  100           100───────  101──  100
         │     │     │    │    │    │              └─range─┘   └┘    └┘
        t0    t1    t2   t3   t4   t5            (t0..t2)   (t3..t4) (t5)

  Rule: consecutive values within precision_epsilon are merged into ONE range.
```

```
  samples:  [100] [100] [100] [101] [101] [100]
               └──── run ────┘   └run┘    └┘
  packed:   ((t0, t2), 100.0)  ((t3,t4), 101.0)  ((t5,t5), 100.0)
```

| Property | Value |
|----------|-------|
| Lossless | No - intermediate timestamps inside a range are dropped on unpack |
| Needs epsilon | Yes |

---

### 2. Mean (`TSPackMeanStrategy { values_compression_percent }`)

Best for: slowly drifting signals where "close enough" to the average is acceptable.

```
  RAW values around ~100 (±5%)              PACKED (1 entry)
  ────────────────────────────             ────────────────────────────
   100  100  102   98  100   99                  avg ≈ 99.83
    ●────●────●────●────●────●        ──►         ●═══════════════●
   t0   t1   t2   t3   t4   t5                  (t0 ──────── t5)
                                                value = mean of window
```

```
  Window mean = 99.83
  Tolerance   = ±5%  →  accepts 95.0 … 104.8

  All samples in range?  YES  →  single packed entry with mean value
  Any outlier?           NO   →  split into multiple entries
```

| Property | Value |
|----------|-------|
| Lossless | No - values replaced by window average |
| Parameter | `values_compression_percent` (e.g. `5` = ±5%) |

---

### 3. Run-length encoding (`TSPackRunLengthStrategy`)

Best for: long stretches of **exactly** the same reading (digital states, idle machines).

```
  RAW                                      PACKED
  ───                                      ──────
  100 ────────────────                     100 ═══════════════  (one run)
  t0  t1  t2  t3  t4  t5                   (t0 ─────────── t5)

  101 ──  101 ──  100                      100══  101══  100
  t6  t7  t8  t9  t10                      (t6─t7)(t8─t9)(t10)
```

```
  RLE vs Similar Values:

  Similar Values:  "100 ≈ 100"  (within epsilon)     → fuzzy match
  Run-length:      "100 == 100" (same IEEE bits)    → exact match
```

| Property | Value |
|----------|-------|
| Lossless (values) | Yes - exact bit pattern preserved |
| Lossless (timestamps) | No - only start/end of each run restored |

---

### 4. Delta (`TSPackDeltaStrategy`)

Best for: smooth signals where each step is a small change from the previous one.

```
  RAW values                             PACKED (deltas)
  ──────────                             ───────────────
  100.0 ──► 101.0 ──► 105.5 ──► 103.0    +0.0   +1.0   +4.5   -2.5
   v0       v1       v2       v3          (raw) (delta)(delta)(delta)
```

```
  Pack:   store v0, then (v1-v0), (v2-v1), (v3-v2)
  Unpack: v0 → v0+d1 → v0+d1+d2 → …

  100.0 ──+1.0──► 101.0 ──+4.5──► 105.5 ──-2.5──► 103.0
          └─delta─┘         └─delta─┘         └─delta─┘
```

| Property | Value |
|----------|-------|
| Lossless (values) | Yes - arithmetic reconstruction |
| Entry count | Same as sample count (one delta per sample) |

---

### 5. Delta-of-Delta (`TSPackDeltaOfDeltaStrategy`)

Best for: signals with smooth acceleration - trends where the *rate of change* itself changes slowly.

```
  RAW values (constant acceleration)       PACKED
  ──────────────────────────────────       ──────
  10 ──► 12 ──► 15 ──► 19 ──► 24           v0   d1   dod   dod   dod
            deltas:  +2   +3   +4   +5     10   +2   +1   +1   +1
            d-o-d:        +1   +1   +1      raw  delta dΔ   dΔ   dΔ
```

```
  Delta layer:        +2      +3      +4      +5
                       └──+1──┘└──+1──┘└──+1──┘
  Delta-of-delta:           +1      +1      +1   ← small numbers, easy to compress

  Pack:   v0, (v1-v0), then (delta_i - delta_{i-1}) for i >= 2
  Unpack: v0 → v0+d1 → … ;  delta_i = delta_{i-1} + dodelta_i
```

| Property | Value |
|----------|-------|
| Lossless (values) | Yes - exact double-precision reconstruction |
| Best when | Delta-of-delta values cluster near zero (smooth trends) |
| Recovery | `TSPackDeltaOfDeltaStrategy::unpack` |

---

### 6. XOR Gorilla (`TSPackXorStrategy`)

Best for: floating-point series with small bit-level changes (Facebook Gorilla TSDB style).

```
  IEEE-754 bits (simplified):

  v0 = 100.0  →  bits: 01000000...
  v1 = 101.0  →  bits: 01000000...   (many bits differ)
                  XOR:  00010110...   ← stored as next entry

  v2 = 105.5  →  bits: 01000000...
                  XOR:  00100101...   ← stored as next entry
```

```
  Pack:
  ┌──────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐
  │ v0   │    │ v0 ^ v1 │    │ v1 ^ v2 │    │ v2 ^ v3 │
  │ raw  │    │  XOR    │    │  XOR    │    │  XOR    │
  └──────┘    └─────────┘    └─────────┘    └─────────┘

  Unpack:  v1 = v0 ^ xor₁ ,  v2 = v1 ^ xor₂ ,  …
```

| Property | Value |
|----------|-------|
| Lossless | Yes - bit-for-bit float recovery via `TSPackXorGorillaStrategy::unpack` |
| Note | `TimeSeriesDataPacker::unpack()` returns encoded XOR values, not originals |

---

### 7. Simple-8b (`TSPackSimple8bStrategy`)

Best for: many small integer deltas - packs dozens of deltas into one 64-bit word.

```
  Step 1 - scale floats to integers (scale = 1 / precision_epsilon):

  values:  100.0 ──► 100.5 ──► 101.0 ──► 102.25
  deltas:         +500        +500        +1250   (milli-units, zigzag-encoded)

  Step 2 - batch integers into 64-bit Simple-8b words:

  ┌──────────────────────────────────────────────────────────────┐
  │ mode │  int │ int │ int │ int │ int │ int │ …  (fits in 64b) │
  │ 4bit │ 4b  │ 4b  │ 4b  │ 4b  │ 4b  │ 4b  │                   │
  └──────────────────────────────────────────────────────────────┘
         ▲
         └── mode selector (how many ints, how many bits each)

  Step 3 - store in packed format:

  ┌─────────────────┐   ┌──────────┐   ┌──────────┐
  │ ANCHOR          │   │ VALUE    │   │ TIME     │
  │ (t0, tN), v0    │   │ words    │   │ words    │
  └─────────────────┘   └──────────┘   └──────────┘
```

```
  64-bit word layout (example mode = 15 integers × 4 bits):

  ┌────┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┐
  │mode│v1│v2│v3│v4│v5│v6│v7│v8│v9│..│..│..│..│..│v15│
  │4bit│4 │4 │4 │4 │4 │4 │4 │4 │4 │  │  │  │  │  │4b │
  └────┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┘
   ▲
   └── top 4 bits = encoding mode
```

| Property | Value |
|----------|-------|
| Lossless | Approximate - within `precision_epsilon` after integer scaling |
| Compression | High when deltas are small integers |
| Recovery | `TSPackSimple8bStrategy::unpack` |

---

### Strategy picker (quick reference)

```
  ┌────────────────────────┬────────────────────────────────────────────────┐
  │ Your data looks like…  │ Start with…                                    │
  ├────────────────────────┼────────────────────────────────────────────────┤
  │ Flat sensor, tiny noise│ Similar Values                                 │
  │ Slow drift around mean │ Mean Strategy                                  │
  │ Long exact plateaus    │ Run-length                                     │
  │ Smooth numeric curve   │ Delta                                          │
  │ Smooth acceleration  │ Delta-of-Delta                                 │
  │ Floats, small changes  │ XOR Gorilla                                    │
  │ Many tiny steps        │ Simple-8b                                      │
  └────────────────────────┴────────────────────────────────────────────────┘

  Lossless value recovery:
    XOR Gorilla  →  TSPackXorGorillaStrategy::unpack
    Delta        →  TSPackDeltaStrategy::unpack
    Delta-of-Delta → TSPackDeltaOfDeltaStrategy::unpack
    Simple-8b    →  TSPackSimple8bStrategy::unpack  (approximate)

  TimeSeriesDataPacker::unpack()  →  expands time ranges only;
                                     does NOT decode XOR / Delta / Delta-of-Delta / Simple-8b payloads
```

---

## API definitions

### Type aliases
- `TSSamples` - `(f64, f64)` - timestamp in seconds, value
- `TSPackedSamples` - `((f64, f64), f64)` - timestamp range `(start, end)` in seconds, packed value

### Enums

#### `TSPackStrategyType`
Available compression strategies (can be chained in `TSPackAttributes::strategy_types`).

> New to the project? See the [Visual guide - strategies explained (ASCII)](#visual-guide--strategies-explained-ascii) section above for diagrams and a strategy picker.

| Variant | Description |
|---------|-------------|
| `TSPackSimilarValuesStrategy` | Groups consecutive samples with equal values (within `precision_epsilon`) into a single time range. Default strategy for repetitive sensor readings. |
| `TSPackMeanStrategy { values_compression_percent: u8 }` | Groups values within +/-N% of the window mean. E.g. `5` packs `100, 102, 98, 100, 99` around their average. |
| `TSPackXorStrategy` | **XOR Gorilla** - lossless bit-level compression inspired by Facebook Gorilla TSDB. First value stored raw; each subsequent value stored as XOR of IEEE-754 bit patterns with the previous value. Use [`TSPackXorGorillaStrategy::unpack`] for lossless recovery. |
| `TSPackDeltaStrategy` | Stores first value raw, then successive deltas (`value - previous`). Lossless for arithmetic differences. |
| `TSPackDeltaOfDeltaStrategy` | **Delta-of-delta** - stores first value raw, first delta, then delta-of-delta for subsequent points. Lossless; ideal for smoothly accelerating signals. Use [`TSPackDeltaOfDeltaStrategy::unpack`] for recovery. |
| `TSPackRunLengthStrategy` | **Run-length encoding (RLE)** - collapses consecutive identical values (exact IEEE-754 bit match) into a single time range. Run length is implicit in `(start_ts, end_ts)`. |
| `TSPackSimple8bStrategy` | **Simple-8b** - variable-bit packing of zigzag-encoded, scaled value deltas and timestamp deltas. First sample stored as anchor; reconstruction is approximate within `precision_epsilon`. Use [`TSPackSimple8bStrategy::unpack`] for recovery. |

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
| `precision_epsilon` | `f64` | Tolerance for value comparison and rounding (ignored for word-exact strategies: XOR Gorilla, Delta, Delta-of-Delta, Simple-8b) |

#### `TimeSeriesDataPacker`
Main packer state object.

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `fn new() -> Self` | Create an empty packer |
| `pack` | `fn pack(&mut self, samples: Vec<TSSamples>, attributes: TSPackAttributes) -> Result<Vec<TSPackedSamples>, TSPackError>` | Sort, window, apply strategies, and store packed output |
| `unpack` | `fn unpack(&self) -> (Option<TSPackAttributes>, Vec<TSSamples>)` | Expand packed ranges to timestamp/value pairs (returns encoded values for XOR/Delta/Delta-of-Delta strategies, not reconstructed originals) |

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

#### `TSPackDeltaOfDeltaStrategy`
Delta-of-delta encoding for float series.

| Method | Signature | Description |
|--------|-----------|-------------|
| `pack` | `fn pack(samples: &[TSSamples]) -> Vec<TSPackedSamples>` | Store first value raw, first delta, then delta-of-delta |
| `unpack` | `fn unpack(packed: &[TSPackedSamples]) -> Vec<TSSamples>` | Reconstruct original values from delta-of-delta chain |

Convenience functions: `delta_of_delta_pack`, `delta_of_delta_unpack`.

#### `TSPackRunLengthStrategy`
Run-length encoding for repeated values.

| Method | Signature | Description |
|--------|-----------|-------------|
| `pack` | `fn pack(samples: &[TSSamples]) -> Vec<TSPackedSamples>` | Collapse consecutive identical values into time ranges |
| `unpack` | `fn unpack(packed: &[TSPackedSamples]) -> Vec<TSSamples>` | Expand each run to start and end timestamp/value pairs |

Convenience functions: `rle_pack`, `rle_unpack` (aliases for the above).

#### `TSPackSimple8bStrategy`
Simple-8b variable-bit integer compression for scaled deltas.

| Method | Signature | Description |
|--------|-----------|-------------|
| `pack` | `fn pack(samples: &[TSSamples], precision_epsilon: f64) -> Vec<TSPackedSamples>` | Anchor first value, encode scaled value/timestamp deltas into Simple-8b words |
| `unpack` | `fn unpack(packed: &[TSPackedSamples], precision_epsilon: f64) -> Vec<TSSamples>` | Decode Simple-8b words and reconstruct approximate samples |

Convenience functions: `simple8b_pack`, `simple8b_unpack`, `simple8b_encode`, `simple8b_decode`, `scale_from_epsilon`.

### Simple-8b - how it works

**Packing:**
1. Store the first sample as an anchor entry `((start_ts, end_ts), first_value)`.
2. Compute value deltas, scale by `1 / precision_epsilon`, zigzag-encode to unsigned integers.
3. Encode timestamp deltas (microseconds) as a second integer stream.
4. Batch each stream into 64-bit Simple-8b words (mode selector in top 4 bits).
5. Store words as `f64` via bit reinterpretation (tagged with `SIMPLE8B_VALUE_WORD_TAG` / `SIMPLE8B_TIME_WORD_TAG`).

**Unpacking:**
1. Read anchor for the first value and start timestamp.
2. Decode value and timestamp word streams.
3. Integrate deltas back into `(timestamp, value)` pairs.

**Example:**
```rust
use time_series_data_packer_rs::*;

let samples = vec![(0.0, 100.0), (1.0, 100.5), (2.0, 101.0)];
let epsilon = TSPackPrecisionDataType::MilisValues.epsilon();

let packed = TSPackSimple8bStrategy::pack(&samples, epsilon);
let recovered = TSPackSimple8bStrategy::unpack(&packed, epsilon);
```

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
- `pack_constant_{size}` - packing constant-value series with Similar Values, Mean, Delta, Delta-of-Delta, XOR Gorilla, Run-length, and Simple-8b strategies
- `delta_of_delta_accelerating_{size}` - Delta-of-Delta pack and unpack on smoothly accelerating values
- `xor_gorilla_incremental_{size}` - XOR Gorilla pack and unpack on slowly changing values
- `run_length_alternating_{size}` - Run-length pack and unpack on alternating-value series
- `simple8b_incremental_{size}` - Simple-8b pack and unpack on slowly changing values

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
