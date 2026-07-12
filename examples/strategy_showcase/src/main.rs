mod data;
mod report;

use std::env;
use std::process;

use time_series_data_packer_rs::{
    TSPackAttributes, TSPackDeltaOfDeltaStrategy, TSPackDeltaStrategy, TSPackPrecisionDataType,
    TSPackRunLengthStrategy, TSPackSimple8bStrategy, TSPackStrategyType,
    TSPackXorGorillaStrategy, TSSamples, TimeSeriesDataPacker,
};

use report::DemoReport;

const USAGE: &str = r#"Strategy showcase - practical IoT and audio compression demos

Usage:
  cargo run -p strategy_showcase -- <strategy>

Strategies:
  similar-values   IoT temperature plateaus (fuzzy merge within epsilon)
  mean             IoT pressure readings clustered around average
  run-length       Digital valve on/off states (exact repeats)
  delta            CNC vibration spectrum (smooth float changes)
  delta-of-delta   Motor RPM ramp with constant acceleration
  xor-gorilla      WAV PCM audio excerpt (bit-level float compression)
  simple8b         WAV PCM audio excerpt (integer delta batches)
  all              Run every demo above

Data files live in examples/data/ - see examples/data/README.md
"#;

fn main() {
    let strategy = env::args().nth(1).unwrap_or_else(|| {
        eprint!("{USAGE}");
        process::exit(1);
    });

    let demos: Vec<(&str, fn())> = vec![
        ("similar-values", demo_similar_values),
        ("mean", demo_mean),
        ("run-length", demo_run_length),
        ("delta", demo_delta),
        ("delta-of-delta", demo_delta_of_delta),
        ("xor-gorilla", demo_xor_gorilla),
        ("simple8b", demo_simple8b),
    ];

    if strategy == "all" {
        for (name, demo) in &demos {
            println!("\n{}", "=".repeat(72));
            println!("  STRATEGY: {name}");
            println!("{}\n", "=".repeat(72));
            demo();
        }
        return;
    }

    if let Some((_, demo)) = demos.iter().find(|(name, _)| *name == strategy) {
        demo();
    } else {
        eprintln!("Unknown strategy: {strategy}\n");
        eprint!("{USAGE}");
        process::exit(1);
    }
}

fn demo_similar_values() {
    let samples = data::load_csv("iot_temperature_sensor.csv").expect("load temperature CSV");
    let epsilon = TSPackPrecisionDataType::IoTSensors.epsilon();

    let attrs = TSPackAttributes {
        strategy_types: vec![TSPackStrategyType::TSPackSimilarValuesStrategy],
        microseconds_time_window: 2_000_000,
        precision_epsilon: epsilon,
    };

    let mut report = DemoReport::new(
        "Similar Values",
        "IoT temperature sensor - Thermomix-style plateaus with slow drift",
        "examples/data/iot_temperature_sensor.csv",
        &samples,
    );

    let packed = pack(&mut report, samples.clone(), attrs);
    report.print_summary(&packed);
    report.print_packed_preview(&packed, 6);

    let (_attrs, expanded) = report.packer.unpack();
    report.print_note(
        "Unpack expands time ranges only; intermediate timestamps inside a plateau are not restored.",
    );
    report.print_samples_preview("Expanded (start/end points)", &expanded, 8);
}

fn demo_mean() {
    let samples = data::load_csv("iot_pressure_noise.csv").expect("load pressure CSV");
    let epsilon = TSPackPrecisionDataType::IoTSensors.epsilon();

    let attrs = TSPackAttributes {
        strategy_types: vec![TSPackStrategyType::TSPackMeanStrategy {
            values_compression_percent: 5,
        }],
        microseconds_time_window: 1_000_000,
        precision_epsilon: epsilon,
    };

    let mut report = DemoReport::new(
        "Mean Strategy",
        "IoT pressure sensor - readings jitter around ~100 kPa within ±5%",
        "examples/data/iot_pressure_noise.csv",
        &samples,
    );

    let packed = pack(&mut report, samples.clone(), attrs);
    report.print_summary(&packed);
    report.print_packed_preview(&packed, 4);
    report.print_note("Values are replaced by window mean - lossy but compact for noisy sensors.");
}

fn demo_run_length() {
    let samples = data::load_csv("iot_valve_state_digital.csv").expect("load valve CSV");

    let attrs = TSPackAttributes {
        strategy_types: vec![TSPackStrategyType::TSPackRunLengthStrategy],
        microseconds_time_window: 2_000_000,
        precision_epsilon: 0.0,
    };

    let mut report = DemoReport::new(
        "Run-length Encoding",
        "Digital valve state - exact 0/1 repeats (machine idle / active)",
        "examples/data/iot_valve_state_digital.csv",
        &samples,
    );

    let packed = pack(&mut report, samples.clone(), attrs);
    report.print_summary(&packed);
    report.print_packed_preview(&packed, 8);

    let recovered = TSPackRunLengthStrategy::unpack(&packed);
    report.print_rle_expansion(&samples, &recovered);
}

fn demo_delta() {
    let samples = data::load_csv("cnc_vibration_spectrum.csv").expect("load spectrum CSV");

    let attrs = TSPackAttributes {
        strategy_types: vec![TSPackStrategyType::TSPackDeltaStrategy],
        microseconds_time_window: 5_000_000,
        precision_epsilon: TSPackPrecisionDataType::WavDerivedAudio.epsilon(),
    };

    let mut report = DemoReport::new(
        "Delta Encoding",
        "CNC machine vibration spectrum - cepstrum levels from 3-axis sound analysis",
        "examples/data/cnc_vibration_spectrum.csv",
        &samples,
    );

    let packed = pack(&mut report, samples.clone(), attrs);
    report.print_summary(&packed);
    report.print_packed_preview(&packed, 6);

    let recovered = TSPackDeltaStrategy::unpack(&packed);
    report.print_recovery_check(&samples, &recovered, |orig, rec| {
        (orig.0 - rec.0).abs() < 1e-9 && (orig.1 - rec.1).abs() < 1e-12
    });
}

fn demo_delta_of_delta() {
    let samples = data::load_csv("iot_motor_rpm_ramp.csv").expect("load motor RPM CSV");

    let attrs = TSPackAttributes {
        strategy_types: vec![TSPackStrategyType::TSPackDeltaOfDeltaStrategy],
        microseconds_time_window: 2_000_000,
        precision_epsilon: 0.0,
    };

    let mut report = DemoReport::new(
        "Delta-of-Delta",
        "Motor RPM ramp - constant acceleration (delta-of-delta stays small)",
        "examples/data/iot_motor_rpm_ramp.csv",
        &samples,
    );

    let packed = pack(&mut report, samples.clone(), attrs);
    report.print_summary(&packed);
    report.print_packed_preview(&packed, 8);

    let recovered = TSPackDeltaOfDeltaStrategy::unpack(&packed);
    report.print_recovery_check(&samples, &recovered, |orig, rec| {
        (orig.0 - rec.0).abs() < 1e-9 && (orig.1 - rec.1).abs() < 1e-12
    });
    report.print_note(
        "Same algorithm family as Gorilla/TimescaleDB timestamp compression - ideal for regular intervals with smooth acceleration.",
    );
}

fn demo_xor_gorilla() {
    let samples = data::load_csv("audio_wav_pcm_excerpt.csv").expect("load audio CSV");
    let epsilon = TSPackPrecisionDataType::WavDerivedAudio.epsilon();

    let attrs = TSPackAttributes {
        strategy_types: vec![TSPackStrategyType::TSPackXorStrategy],
        microseconds_time_window: 1_000_000,
        precision_epsilon: epsilon,
    };

    let mut report = DemoReport::new(
        "XOR Gorilla",
        "WAV PCM audio excerpt (8 kHz) - consecutive float samples XOR-compressed",
        "examples/data/audio_wav_pcm_excerpt.csv",
        &samples,
    );

    let packed = pack(&mut report, samples.clone(), attrs);
    report.print_summary(&packed);
    report.print_packed_preview(&packed, 6);

    let recovered = TSPackXorGorillaStrategy::unpack(&packed);
    report.print_recovery_check(&samples, &recovered, |orig, rec| {
        (orig.0 - rec.0).abs() < 1e-9 && orig.1.to_bits() == rec.1.to_bits()
    });
}

fn demo_simple8b() {
    let samples = data::load_csv("audio_wav_pcm_excerpt.csv").expect("load audio CSV");
    let epsilon = TSPackPrecisionDataType::WavDerivedAudio.epsilon();

    let attrs = TSPackAttributes {
        strategy_types: vec![TSPackStrategyType::TSPackSimple8bStrategy],
        microseconds_time_window: 1_000_000,
        precision_epsilon: epsilon,
    };

    let mut report = DemoReport::new(
        "Simple-8b",
        "WAV PCM audio excerpt - scaled deltas batched into 64-bit words",
        "examples/data/audio_wav_pcm_excerpt.csv",
        &samples,
    );

    let packed = pack(&mut report, samples.clone(), attrs);
    report.print_summary(&packed);
    report.print_packed_preview(&packed, 8);

    let recovered = TSPackSimple8bStrategy::unpack(&packed, epsilon);
    report.print_recovery_check(&samples, &recovered, |orig, rec| {
        (orig.0 - rec.0).abs() < 1e-6 && (orig.1 - rec.1).abs() < epsilon
    });
}

fn pack(report: &mut DemoReport, samples: Vec<TSSamples>, attrs: TSPackAttributes) -> Vec<time_series_data_packer_rs::TSPackedSamples> {
    report
        .packer
        .pack(samples, attrs)
        .expect("pack should succeed")
}
