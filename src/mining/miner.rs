use crate::blockchain::block::Block;
use crate::fractal::FractalType;

pub struct Miner;

impl Miner {
    /// Mines a block using a proof-of-work algorithm that involves generating fractals.
    ///
    /// The algorithm requires finding a nonce that, when used as a seed for the fractal,
    /// produces a block hash that starts with a certain number of zeros.
    pub fn mine_block(difficulty: usize, fractal_type: FractalType, mut block: Block) -> Block {
        let prefix = "0".repeat(difficulty);

        loop {
            let mut current_fractal_type = fractal_type.clone();
            match &mut current_fractal_type {
                FractalType::Sierpinski { seed, .. } => *seed = block.nonce,
                FractalType::Mandelbrot { seed, .. } => *seed = block.nonce,
                FractalType::Julia { seed, .. } => *seed = block.nonce,
            }

            block.fractal = current_fractal_type.generate();

            let hash = block.calculate_hash();
            if hash.starts_with(&prefix) {
                block.hash = hash;
                return block;
            }
            block.nonce += 1;
        }
    }
}
