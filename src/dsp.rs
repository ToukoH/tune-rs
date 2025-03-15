use rustfft::{FftPlanner, num_complex::Complex};
use std::f32::consts::PI;

pub fn compute_fft(input: &[f32]) -> Vec<f32> {
    let len = input.len();

    let windowed: Vec<f32> = input
        .iter()
        .enumerate()
        .map(|(n, &x)| {
            let w = 0.5 * (1.0 - (2.0 * PI * n as f32 / (len as f32 - 1.0)).cos());
            x * w
        })
        .collect();

    let mut buffer: Vec<Complex<f32>> = windowed
        .iter()
        .map(|&x| Complex { re: x, im: 0.0 })
        .collect();
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(len);
    fft.process(&mut buffer);
    buffer.iter().map(|c| c.norm()).collect()
}
