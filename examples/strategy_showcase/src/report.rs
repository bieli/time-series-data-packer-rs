use time_series_data_packer_rs::{TSPackedSamples, TSSamples, TimeSeriesDataPacker};

pub struct DemoReport {
    pub packer: TimeSeriesDataPacker,
    strategy_name: &'static str,
    description: &'static str,
    data_file: &'static str,
    raw_count: usize,
    time_span_s: f64,
    value_min: f64,
    value_max: f64,
}

impl DemoReport {
    pub fn new(
        strategy_name: &'static str,
        description: &'static str,
        data_file: &'static str,
        samples: &[TSSamples],
    ) -> Self {
        let (value_min, value_max) = samples
            .iter()
            .map(|(_, v)| *v)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), v| {
                (min.min(v), max.max(v))
            });

        let time_span_s = if samples.len() >= 2 {
            samples.last().unwrap().0 - samples.first().unwrap().0
        } else {
            0.0
        };

        Self {
            packer: TimeSeriesDataPacker::new(),
            strategy_name,
            description,
            data_file,
            raw_count: samples.len(),
            time_span_s,
            value_min,
            value_max,
        }
    }

    pub fn print_summary(&self, packed: &[TSPackedSamples]) {
        let ratio = if packed.is_empty() {
            0.0
        } else {
            self.raw_count as f64 / packed.len() as f64
        };

        println!("Strategy:     {}", self.strategy_name);
        println!("Dataset:      {}", self.description);
        println!("Source file:  {}", self.data_file);
        println!("Raw samples:  {}", self.raw_count);
        println!("Time span:    {:.3} s", self.time_span_s);
        println!("Value range:  {:.4} … {:.4}", self.value_min, self.value_max);
        println!("Packed entries: {}", packed.len());
        println!("Compression:  {:.1}x fewer entries ({:.1}%)", ratio, 100.0 / ratio.max(1.0));
    }

    pub fn print_packed_preview(&self, packed: &[TSPackedSamples], limit: usize) {
        println!("\nPacked preview (first {limit}):");
        for ((start, end), value) in packed.iter().take(limit) {
            if start.is_infinite() || start.is_nan() {
                let tag = if *start == f64::NEG_INFINITY {
                    "VALUE_WORD"
                } else if *start == f64::INFINITY {
                    "TIME_WORD"
                } else {
                    "WORD"
                };
                println!("  ([{tag}], bits=0x{:016x})", value.to_bits());
                continue;
            }
            if (end - start).abs() < 1e-12 {
                println!("  (({start:.6}, {end:.6}), {value:.6})");
            } else {
                println!("  (({start:.6} … {end:.6}), {value:.6})");
            }
        }
        if packed.len() > limit {
            println!("  … {} more entries", packed.len() - limit);
        }
    }

    pub fn print_samples_preview(&self, label: &str, samples: &[TSSamples], limit: usize) {
        println!("\n{label} (first {limit}):");
        for (ts, value) in samples.iter().take(limit) {
            println!("  ({ts:.6}, {value:.6})");
        }
    }

    pub fn print_rle_expansion(&self, original: &[TSSamples], expanded: &[TSSamples]) {
        println!(
            "\nRLE expansion: {} runs → {} boundary points (from {} raw samples)",
            expanded.len(),
            expanded.len(),
            original.len()
        );
        let values_match = expanded.iter().all(|(ts, val)| {
            original
                .iter()
                .any(|(ots, oval)| (ots - ts).abs() < 1e-9 && oval.to_bits() == val.to_bits())
        });
        println!(
            "Recovery:     {}",
            if values_match {
                "OK - all expanded points match original samples"
            } else {
                "MISMATCH"
            }
        );
        self.print_note(
            "RLE stores run ranges; unpack restores start/end timestamps only, not every sample in between.",
        );
    }

    pub fn print_recovery_check<F>(&self, original: &[TSSamples], recovered: &[TSSamples], matches: F)
    where
        F: Fn(&TSSamples, &TSSamples) -> bool,
    {
        let ok = original.len() == recovered.len()
            && original
                .iter()
                .zip(recovered.iter())
                .all(|(a, b)| matches(a, b));

        println!(
            "\nRecovery:     {}",
            if ok { "OK - roundtrip successful" } else { "MISMATCH - check tolerance" }
        );
    }

    pub fn print_note(&self, note: &str) {
        println!("\nNote: {note}");
    }
}
