use serde::{Serialize, Deserialize};

/// Represents a Sierpinski triangle fractal.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FractalTriangle {
    /// The depth of the fractal.
    pub depth: usize,
    /// The vertices of the triangles that make up the fractal.
    pub vertices: Vec<(f64, f64)>,
}

impl FractalTriangle {
    /// Generates a new `FractalTriangle` of a given depth.
    pub fn generate(depth: usize) -> Self {
        let mut vertices = Vec::new();
        let initial_triangle = [(0.0, 0.0), (1.0, 0.0), (0.5, 0.866)];
        Self::subdivide(&mut vertices, depth, initial_triangle[0], initial_triangle[1], initial_triangle[2]);
        FractalTriangle { depth, vertices }
    }

    /// Recursively subdivides a triangle to generate the fractal.
    fn subdivide(vertices: &mut Vec<(f64, f64)>, depth: usize, p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) {
        if depth == 0 {
            // Base case: add the triangle's vertices to the list.
            vertices.push(p1);
            vertices.push(p2);
            vertices.push(p3);
        } else {
            // Recursive step: calculate midpoints and subdivide.
            let m12 = ((p1.0 + p2.0) / 2.0, (p1.1 + p2.1) / 2.0);
            let m23 = ((p2.0 + p3.0) / 2.0, (p2.1 + p3.1) / 2.0);
            let m13 = ((p1.0 + p3.0) / 2.0, (p1.1 + p3.1) / 2.0);

            Self::subdivide(vertices, depth - 1, p1, m12, m13);
            Self::subdivide(vertices, depth - 1, m12, p2, m23);
            Self::subdivide(vertices, depth - 1, m13, m23, p3);
        }
    }
}
