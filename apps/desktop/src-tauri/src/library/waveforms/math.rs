pub(crate) fn waveform_peak_rms_bins(
    source_samples: &[f64],
    sample_count: usize,
) -> (Vec<f64>, Vec<f64>) {
    if source_samples.is_empty() {
        return (vec![0.0; sample_count], vec![0.0; sample_count]);
    }

    let mut peak_samples = Vec::with_capacity(sample_count);
    let mut rms_samples = Vec::with_capacity(sample_count);
    for index in 0..sample_count {
        let start = index * source_samples.len() / sample_count;
        let end = ((index + 1) * source_samples.len() / sample_count)
            .max(start + 1)
            .min(source_samples.len());
        let mut peak = 0.0_f64;
        let mut sum_squares = 0.0_f64;
        for sample in &source_samples[start..end] {
            let amplitude = sample.abs().min(1.0);
            peak = peak.max(amplitude);
            sum_squares += amplitude * amplitude;
        }
        peak_samples.push(peak);
        rms_samples.push((sum_squares / (end - start) as f64).sqrt());
    }

    (peak_samples, rms_samples)
}

pub(crate) fn waveform_tile_sample_count(
    duration_ms: i64,
    bin_ms: i64,
    source_is_empty: bool,
) -> usize {
    let sample_count = ((duration_ms.max(1) + bin_ms.max(1) - 1) / bin_ms.max(1)) as usize;
    if source_is_empty {
        sample_count
    } else {
        sample_count.max(2)
    }
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
