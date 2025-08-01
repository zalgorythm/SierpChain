use serde::{Serialize, Deserialize};
use super::utils::Lcg;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Mandelbrot {
    pub width: usize,
    pub height: usize,
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub max_iterations: u32,
    pub seed: u64,
    pub data: Vec<u32>,
}

impl Mandelbrot {
    pub fn generate(
        width: usize,
        height: usize,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        max_iterations: u32,
        seed: u64,
    ) -> Self {
        let mut data = vec![0; width * height];
        let mut rng = Lcg::new(seed);
        let perturbation_scale = 0.001 / (max_iterations as f64);

        for py in 0..height {
            for px in 0..width {
                let x0 = x_min + (px as f64 / width as f64) * (x_max - x_min);
                let y0 = y_min + (py as f64 / height as f64) * (y_max - y_min);
                let mut x = 0.0;
                let mut y = 0.0;
                let mut iteration = 0;
                while x * x + y * y <= 4.0 && iteration < max_iterations {
                    let xtemp = x * x - y * y + x0 + rng.next_float() * perturbation_scale;
                    y = 2.0 * x * y + y0 + rng.next_float() * perturbation_scale;
                    x = xtemp;
                    iteration += 1;
                }
                data[py * width + px] = iteration;
            }
        }
        Mandelbrot {
            width,
            height,
            x_min,
            x_max,
            y_min,
            y_max,
            max_iterations,
            seed,
            data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mandelbrot_generation() {
        let mandelbrot = Mandelbrot::generate(10, 10, -2.0, 1.0, -1.5, 1.5, 100, 0);
        assert_eq!(mandelbrot.data.len(), 100);

        // Test a point in the set (center of the main cardioid)
        let center_x = ((-0.25 - (-2.0)) / (1.0 - (-2.0)) * 10.0) as usize;
        let center_y = ((0.0 - (-1.5)) / (1.5 - (-1.5)) * 10.0) as usize;
        assert_eq!(mandelbrot.data[center_y * 10 + center_x], 100);

        // Test a point outside the set
        let outside_x = 9;
        let outside_y = 9;
        assert!(mandelbrot.data[outside_y * 10 + outside_x] < 100);
    }
}
