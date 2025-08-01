/// A simple Linear Congruential Generator for pseudo-random numbers.
pub struct Lcg {
    state: u64,
}

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Lcg { state: seed }
    }

    pub fn next(&mut self) -> u64 {
        // Parameters from POSIX standard for rand()
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        self.state
    }

    /// Returns a float between -1.0 and 1.0
    pub fn next_float(&mut self) -> f64 {
        (self.next() % 2001) as f64 / 1000.0 - 1.0
    }
}
