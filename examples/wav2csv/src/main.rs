use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use hound::{WavReader, SampleFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: wav2csv <input.wav> <output.csv>");
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];

    let mut reader = WavReader::open(input_path)?;
    let spec = reader.spec();

    let sample_rate = spec.sample_rate as f64;
    let channels = spec.channels as usize;

    if channels != 1 {
        eprintln!("Warning: WAV has {} channels. Only channel 0 will be exported.", channels);
    }

    let mut writer = BufWriter::new(File::create(output_path)?);
    writeln!(writer, "timestamp_us,value")?;

    let mut sample_index: u64 = 0;

    match spec.sample_format {
        SampleFormat::Int => {
            let max_val = (1i64 << (spec.bits_per_sample - 1)) as f64;

            for sample in reader.samples::<i32>() {
                let s = sample? as f64 / max_val;

                let timestamp_us = (sample_index as f64 / sample_rate) * 1_000_000.0;
                writeln!(writer, "{:.0},{:.12}", timestamp_us, s)?;

                sample_index += 1;
            }
        }
        SampleFormat::Float => {
            for sample in reader.samples::<f32>() {
                let s = sample? as f64;

                let timestamp_us = (sample_index as f64 / sample_rate) * 1_000_000.0;
                writeln!(writer, "{:.0},{:.12}", timestamp_us, s)?;

                sample_index += 1;
            }
        }
    }

    writer.flush()?;
    Ok(())
}

