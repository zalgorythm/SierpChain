use serde::{Serialize, Deserialize};
pub mod sierpinski;
pub mod mandelbrot;
pub mod utils;

use self::sierpinski::Sierpinski;
use self::mandelbrot::Mandelbrot;

/// An enum to hold the data for different fractal types.
/// This will be stored in the block.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum FractalData {
    Sierpinski(Sierpinski),
    Mandelbrot(Mandelbrot),
}

/// An enum to represent the different types of fractals that can be generated.
/// This will be used in the mining request.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum FractalType {
    Sierpinski { depth: usize, seed: u64 },
    Mandelbrot {
        width: usize,
        height: usize,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        max_iterations: u32,
        seed: u64,
    },
}

impl FractalType {
    pub fn generate(&self) -> FractalData {
        match self {
            FractalType::Sierpinski { depth, seed } => {
                FractalData::Sierpinski(Sierpinski::generate(*depth, *seed))
            }
            FractalType::Mandelbrot {
                width,
                height,
                x_min,
                x_max,
                y_min,
                y_max,
                max_iterations,
                seed,
            } => FractalData::Mandelbrot(Mandelbrot::generate(
                *width,
                *height,
                *x_min,
                *x_max,
                *y_min,
                *y_max,
                *max_iterations,
                *seed,
            )),
        }
    }
}
