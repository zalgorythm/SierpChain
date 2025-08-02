use serde::{Serialize, Deserialize};
use super::utils::Lcg;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Julia {
    pub width: usize,
    pub height: usize,
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub c_real: f64,
    pub c_imag: f64,
    pub max_iterations: u32,
    pub seed: u64,
    pub data: Vec<u32>,
}

impl Julia {
    pub fn generate(
        width: usize,
        height: usize,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        c_real: f64,
        c_imag: f64,
        max_iterations: u32,
        seed: u64,
    ) -> Self {
        let mut data = vec![0; width * height];
        let mut rng = Lcg::new(seed);
        let perturbation_scale = 0.001 / (max_iterations as f64);

        for py in 0..height {
            for px in 0..width {
                let mut x = x_min + (px as f64 / width as f64) * (x_max - x_min);
                let mut y = y_min + (py as f64 / height as f64) * (y_max - y_min);
                let mut iteration = 0;
                while x * x + y * y <= 4.0 && iteration < max_iterations {
                    let xtemp = x * x - y * y + c_real + rng.next_float() * perturbation_scale;
                    y = 2.0 * x * y + c_imag + rng.next_float() * perturbation_scale;
                    x = xtemp;
                    iteration += 1;
                }
                data[py * width + px] = iteration;
            }
        }
        Julia {
            width,
            height,
            x_min,
            x_max,
            y_min,
            y_max,
            c_real,
            c_imag,
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
    fn test_julia_generation() {
        // c = -0.8 + 0.156i
        let julia = Julia::generate(10, 10, -1.5, 1.5, -1.5, 1.5, -0.8, 0.156, 100, 0);
        assert_eq!(julia.data.len(), 100);

        // Test a point that should escape quickly
        let outside_x = 9;
        let outside_y = 9;
        assert!(julia.data[outside_y * 10 + outside_x] < 100);

        // Test a point that should be in the set
        let inside_x = 5;
        let inside_y = 5;
        assert_eq!(julia.data[inside_y * 10 + inside_x], 100);
    }
}
