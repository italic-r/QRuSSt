/// Window functions for FFT processing
///
/// Math borrowed from Onno Hoekstra (PA2OHH)


use std::f32::consts::PI;


/// No window shape
pub (crate) fn rectangle(window_length: usize) -> Vec<f32> {
    vec![1.; window_length]
}

/// Cosine
pub (crate) fn cosine(window_length: usize) -> Vec<f32> {
    (0..window_length).map(|n|
        (PI * n as f32 / (window_length - 1) as f32).sin() * 1.571
    ).collect()
}

/// Triangular
pub (crate) fn triangle(window_length: usize) -> Vec<f32> {
    (0..window_length).map(|n|
        (2. / window_length as f32) * ((window_length as f32 / 2.) - (n as f32 - (window_length - 1) as f32 / 2.).abs()) * 2.
    ).collect()
}

/// Hamming
pub (crate) fn hamming(window_length: usize) -> Vec<f32> {
    (0..window_length).map(|n|
        (0.53836 - (0.46164 * (2. * PI * n as f32 / (window_length - 1) as f32).cos())) * 2.
    ).collect()
}

/// Hann
pub (crate) fn hann(window_length: usize) -> Vec<f32> {
    (0..window_length).map(|n|
        (0.5 - (0.5 * (2. * PI * n as f32 / (window_length - 1) as f32).cos())) * 2.
    ).collect()
}

/// Blackman
pub (crate) fn blackman(window_length: usize) -> Vec<f32> {
    (0..window_length).map(|n|
        (0.42659 - (0.496560 * (2. * PI * n as f32 / (window_length - 1) as f32).cos() +
                    0.076849 * (4. * PI * n as f32 / (window_length - 1) as f32).cos())) * 2.381
    ).collect()
}

/// Nuttall
pub (crate) fn nuttall(window_length: usize) -> Vec<f32> {
    (0..window_length).map(|n|
        (0.355768 - (0.487396 * (2. * PI * n as f32 / (window_length - 1) as f32).cos() +
                     0.144320 * (4. * PI * n as f32 / (window_length - 1) as f32).cos() -
                     0.012604 * (6. * PI * n as f32 / (window_length - 1) as f32).cos())) * 2.811
    ).collect()
}

/// Flat top
pub (crate) fn flat(window_length: usize) -> Vec<f32> {
    (0..window_length).map(|n|
        (1. - (1.930 * (2. * PI * n as f32 / (window_length - 1) as f32).cos() +
               1.290 * (4. * PI * n as f32 / (window_length - 1) as f32).cos() -
               0.388 * (6. * PI * n as f32 / (window_length - 1) as f32).cos() +
               0.032 * (8. * PI * n as f32 / (window_length - 1) as f32).cos()))
    ).collect()
}
