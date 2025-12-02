use crate::TSSamples;

pub fn split_into_windows(samples: &[TSSamples], micro_window: u64) -> Vec<Vec<TSSamples>> {
    if samples.is_empty() {
        return Vec::new();
    }

    let mut windows: Vec<Vec<TSSamples>> = Vec::new();
    let mut current: Vec<TSSamples> = Vec::new();

    let mut window_start_ts = samples[0].0;
    let window_len_seconds = (micro_window as f64) / 1_000_000.0;

    for &(ts, val) in samples {
        if ts - window_start_ts <= window_len_seconds {
            current.push((ts, val));
        } else {
            if !current.is_empty() {
                windows.push(current.clone());
                current.clear();
            }
            window_start_ts = ts;
            current.push((ts, val));
        }
    }

    if !current.is_empty() {
        windows.push(current);
    }

    windows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windowing_microseconds() {
        let samples = vec![
            (0.00, 1.0),
            (0.05, 1.0),
            (0.10, 2.0),
            (0.15, 2.0),
            (0.21, 3.0),
        ];

        let windows = split_into_windows(&samples, 100_000);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].len(), 3);
        assert_eq!(windows[1].len(), 2);
    }
}
