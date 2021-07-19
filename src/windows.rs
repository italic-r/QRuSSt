/// Window functions for FFT processing


pub (crate) fn hann_window(window_length: usize) -> Vec<f32> {
    (0..window_length).map(|n|
        (0.5 - (0.5 * (PI * n as f32 * 2. / (window_length as f32 - 1.)).sin())) * 2.
    ).collect()
}
