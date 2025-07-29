use serde::{Serialize, Deserialize};
use crate::fractal::FractalTriangle;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub prev_hash: String,
    pub nonce: u64,
    pub fractal: FractalTriangle,
    pub hash: String,
}

impl Block {
    pub fn new(index: u64, timestamp: u64, prev_hash: String, nonce: u64, fractal: FractalTriangle, hash: String) -> Self {
        Block { index, timestamp, prev_hash, nonce, fractal, hash }
    }
}
