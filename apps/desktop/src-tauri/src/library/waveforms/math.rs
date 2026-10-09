pub(crate) fn waveform_peak_rms_time_bins(
    source_samples: &[f64],
    duration_ms: i64,
    bin_ms: i64,
) -> (Vec<f64>, Vec<f64>) {
    let duration_ms = duration_ms.max(1) as usize;
    let bin_ms = bin_ms.max(1) as usize;
    let count = duration_ms.div_ceil(bin_ms);
    let mut peaks = Vec::with_capacity(count);
    let mut rms = Vec::with_capacity(count);
    for index in 0..count {
        // FFmpeg returns 48 kHz PCM, including timestamp gaps and tail padding.
        // Keep the last partial bin short instead of redistributing every bin.
        let start = (index * bin_ms * 48).min(source_samples.len());
        let end = (((index + 1) * bin_ms).min(duration_ms) * 48).min(source_samples.len());
        let (peak, value) = peak_rms(&source_samples[start..end]);
        peaks.push(peak);
        rms.push(value);
    }
    (peaks, rms)
}

fn peak_rms(samples: &[f64]) -> (f64, f64) {
    let mut peak = 0.0_f64;
    let mut sum_squares = 0.0_f64;
    for sample in samples {
        let amplitude = sample.abs().min(1.0);
        peak = peak.max(amplitude);
        sum_squares += amplitude * amplitude;
    }
    (peak, (sum_squares / samples.len().max(1) as f64).sqrt())
}

pub(crate) fn waveform_bins(source_samples: &[f64], sample_count: usize) -> Vec<f64> {
    if source_samples.is_empty() {
        return vec![0.0; sample_count];
    }

    (0..sample_count)
        .map(|index| {
            let start = index * source_samples.len() / sample_count;
            let end = ((index + 1) * source_samples.len() / sample_count)
                .max(start + 1)
                .min(source_samples.len());
            source_samples[start..end]
                .iter()
                .copied()
                .fold(0.0, f64::max)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_tail_does_not_move_the_time_bin_boundary() {
        let mut samples = vec![0.; 7 * 48];
        samples[220] = 0.5;
        let (peaks, rms) = waveform_peak_rms_time_bins(&samples, 7, 5);
        assert_eq!(peaks, vec![0.5, 0.]);
        assert!((rms[0] - (0.25_f64 / 240.).sqrt()).abs() < 1e-10);
        assert_eq!(rms[1], 0.);
        assert_eq!(waveform_peak_rms_time_bins(&[], 7, 5).0, vec![0., 0.]);
    }
}
