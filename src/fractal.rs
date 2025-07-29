use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FractalTriangle {
    pub depth: usize,
    pub vertices: Vec<(f64, f64)>,
}

impl FractalTriangle {
    pub fn generate(depth: usize) -> Self {
        let vertices = vec![(0.0, 0.0), (1.0, 0.0), (0.5, 0.866)];
        FractalTriangle { depth, vertices }
    }
}
