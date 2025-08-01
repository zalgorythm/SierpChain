use serde::{Serialize, Deserialize};
use super::utils::Lcg;

/// Represents a Sierpinski triangle fractal.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Sierpinski {
    /// The depth of the fractal.
    pub depth: usize,
    /// The seed used to generate the fractal.
    pub seed: u64,
    /// The vertices of the triangles that make up the fractal.
    pub vertices: Vec<(f64, f64)>,
}

impl Sierpinski {
    /// Generates a new `Sierpinski` fractal of a given depth and seed.
    pub fn generate(depth: usize, seed: u64) -> Self {
        let mut vertices = Vec::new();
        let initial_triangle = [(0.0, 0.0), (1.0, 0.0), (0.5, 0.866)];
        let mut rng = Lcg::new(seed);
        Self::subdivide(&mut vertices, depth, initial_triangle[0], initial_triangle[1], initial_triangle[2], &mut rng);
        Sierpinski { depth, seed, vertices }
    }

    /// Recursively subdivides a triangle to generate the fractal.
    fn subdivide(vertices: &mut Vec<(f64, f64)>, depth: usize, p1: (f64, f64), p2: (f64, f64), p3: (f64, f64), rng: &mut Lcg) {
        if depth == 0 {
            // Base case: add the triangle's vertices to the list.
            vertices.push(p1);
            vertices.push(p2);
            vertices.push(p3);
        } else {
            // Recursive step: calculate midpoints and subdivide.
            let perturbation_scale = 0.05 / (depth as f64);

            let m12 = (
                (p1.0 + p2.0) / 2.0 + rng.next_float() * perturbation_scale,
                (p1.1 + p2.1) / 2.0 + rng.next_float() * perturbation_scale,
            );
            let m23 = (
                (p2.0 + p3.0) / 2.0 + rng.next_float() * perturbation_scale,
                (p2.1 + p3.1) / 2.0 + rng.next_float() * perturbation_scale,
            );
            let m13 = (
                (p1.0 + p3.0) / 2.0 + rng.next_float() * perturbation_scale,
                (p1.1 + p3.1) / 2.0 + rng.next_float() * perturbation_scale,
            );

            Self::subdivide(vertices, depth - 1, p1, m12, m13, rng);
            Self::subdivide(vertices, depth - 1, m12, p2, m23, rng);
            Self::subdivide(vertices, depth - 1, m13, m23, p3, rng);
        }
    }
}
