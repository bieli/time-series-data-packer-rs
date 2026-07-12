# Example datasets

Practical time-series samples for strategy demonstrations. All files use CSV with a header row.

| File | Source | Samples | Best strategies |
|------|--------|---------|-----------------|
| `iot_temperature_sensor.csv` | Real Thermomix-style temperature readings (°C) | 54 | Similar Values, Run-length |
| `iot_valve_state_digital.csv` | Synthetic digital valve 0/1 states | 18 | Run-length |
| `iot_motor_rpm_ramp.csv` | Synthetic motor RPM with constant acceleration | 16 | Delta-of-Delta, Delta |
| `iot_pressure_noise.csv` | Synthetic pressure sensor jitter around 100 kPa | 16 | Mean, Similar Values |
| `cnc_vibration_spectrum.csv` | CNC 3-axis sound cepstrum levels (from `assets/`) | 120 | Delta, XOR Gorilla |
| `audio_wav_pcm_excerpt.csv` | First 500 PCM samples from 8 kHz WAV (via `wav2csv`) | 500 | XOR Gorilla, Simple-8b, Delta |

## CSV formats

**Microsecond timestamps** (IoT sensors):
```csv
timestamp_us,value
1431142,26.5
```

**Microsecond timestamps** (audio PCM):
```csv
timestamp_us,value
0,0.000000981590
125,0.000000590572
```

## Regenerating audio data

```bash
# Convert WAV to CSV (requires examples/wav2csv)
cargo run -p wav2csv -- assets/your_file.wav examples/data/audio_wav_pcm_excerpt.csv
head -n 501 examples/data/audio_wav_pcm_excerpt.csv > examples/data/audio_wav_pcm_excerpt.csv
```

## Regenerating CNC spectrum data

Derived from `assets/cnc_machine_3_axis_sounds1.spectrum_cepstrum_squares_window.size_512.txt` - lag column converted to `timestamp_us`, cepstrum level as `value`.
