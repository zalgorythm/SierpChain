use crate::blockchain::block::Block;
use crate::core::fractal::FractalTriangle;

pub struct Miner;

impl Miner {
    /// Mines a block using a proof-of-work algorithm that involves generating fractals.
    ///
    /// The algorithm requires finding a nonce that, when used as a seed for the fractal,
    /// produces a block hash that starts with a certain number of zeros.
    pub fn mine_block(difficulty: usize, fractal_depth: usize, mut block: Block) -> Block {
        let prefix = "0".repeat(difficulty);

        loop {
            block.fractal = FractalTriangle::generate(fractal_depth, block.nonce);
            let hash = block.calculate_hash();
            if hash.starts_with(&prefix) {
                block.hash = hash;
                return block;
            }
            block.nonce += 1;
        }
    }
}
