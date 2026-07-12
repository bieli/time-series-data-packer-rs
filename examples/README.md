# Examples

Runnable demonstrations of every packing strategy using **practical IoT sensor** and **audio/WAV-derived** datasets.

## Quick start

```bash
# Run one strategy
cargo run -p strategy_showcase -- similar-values
cargo run -p strategy_showcase -- delta-of-delta
cargo run -p strategy_showcase -- xor-gorilla

# Run all demos
cargo run -p strategy_showcase -- all
```

Run from the repository root. Each demo prints dataset info, compression ratio, packed preview, and recovery status.

## Strategy → dataset mapping

| Command | Strategy | Dataset | Real-world scenario |
|---------|----------|---------|-------------------|
| `similar-values` | Similar Values | `iot_temperature_sensor.csv` | Cooking appliance temperature plateaus |
| `mean` | Mean (±5%) | `iot_pressure_noise.csv` | Pressure sensor noise around setpoint |
| `run-length` | Run-length | `iot_valve_state_digital.csv` | PLC valve open/closed states |
| `delta` | Delta | `cnc_vibration_spectrum.csv` | CNC machine vibration cepstrum |
| `delta-of-delta` | Delta-of-Delta | `iot_motor_rpm_ramp.csv` | Motor RPM ramp / constant acceleration |
| `xor-gorilla` | XOR Gorilla | `audio_wav_pcm_excerpt.csv` | WAV PCM float samples (8 kHz excerpt) |
| `simple8b` | Simple-8b | `audio_wav_pcm_excerpt.csv` | WAV PCM - batched integer deltas |

Sample data lives in [`data/`](data/README.md).

## Other examples

| Example | Description |
|---------|-------------|
| [`strategy_showcase/`](strategy_showcase/) | **Main demo** - one binary, all strategies |
| [`wav2csv/`](wav2csv/) | Convert WAV files to `(timestamp_us, value)` CSV |
| [`csv_to_strategies_discovery/`](csv_to_strategies_discovery/) | Batch strategy comparison on custom CSV input |

---

## Delta-of-Delta in open-source time-series systems

Delta-of-delta stores the **second derivative** of a series: the change in deltas between consecutive values. It shines when data arrives at **regular intervals** with **smooth, predictable rate-of-change** - timestamps in Gorilla, integer counters, motor RPM ramps.

### Systems that use it (directly or as part of a codec chain)

| Project | Type | How delta-of-delta is used |
|---------|------|---------------------------|
| **[Facebook Gorilla](https://vldb.org/pvldb/vol8/p1816-teller.pdf)** (paper) | TSDB design | **Timestamps** compressed with delta-of-delta + variable-length encoding; **floats** use XOR (not delta-of-delta) |
| **[InfluxDB](https://github.com/influxdata/influxdb)** (engine) | Open-source TSDB | Gorilla-inspired block encoding; delta-of-delta on timestamps in time-series blocks |
| **[TimescaleDB](https://github.com/timescale/timescaledb)** | PostgreSQL extension | `deltadelta` compressor for **integers, timestamps, booleans** - zigzag + Simple-8b + RLE chain |
| **[DeltaX (pg_deltax)](https://github.com/xataio/deltax)** | PostgreSQL extension | **Gorilla delta-of-delta** for `timestamp` / `date` columns; XOR for floats |
| **[QuestDB](https://github.com/questdb/questdb)** | Open-source TSDB | Gorilla-style timestamp compression in column storage |
| **[Apache IoTDB](https://github.com/apache/iotdb)** | IoT time-series DB | Delta encoding family for numeric/time columns in TsFile format |
| **[ClickHouse](https://github.com/ClickHouse/ClickHouse)** | OLAP / analytics | `DoubleDelta` codec - delta-of-delta for integer sequences (timestamps, counters) |
| **[deltax](https://github.com/xataio/deltax)** / industry pattern | Columnar | Often combined: **delta-of-delta → zigzag → Simple-8b → RLE** (same pipeline as this crate's Simple-8b module) |

### Key insight

> **Delta-of-delta is almost never used alone on floats.** Production systems apply it to **timestamps** and **integer-like values**, then use **XOR Gorilla** or **delta** for floating-point measurements. This crate's `TSPackDeltaOfDeltaStrategy` applies the algorithm to **float values** directly - useful for smoothly accelerating sensor readings (RPM, position, integrators).

### Further reading

- [TimescaleDB compression methods](https://github.com/timescale/docs/blob/latest/use-timescale/hypercore/compression-methods.md)
- [Gorilla VLDB 2015 paper](https://vldb.org/pvldb/vol8/p1816-teller.pdf) - Section 4.1.1 (timestamp delta-of-delta)
- [TimescaleDB `deltadelta.h` source](https://github.com/timescale/timescaledb/blob/master/tsl/src/compression/algorithms/deltadelta.h)
