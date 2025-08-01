use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use crate::fractal::FractalData;
use crate::core::transaction::{Transaction};

/// Represents a block in the SierpChain.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Block {
    pub index: u64,
    pub timestamp: i64,
    pub fractal: FractalData,
    pub transactions: Vec<Transaction>,
    pub previous_hash: String,
    pub hash: String,
    pub nonce: u64,
}

impl Block {
    /// Calculates the SHA-256 hash of the block.
    pub fn calculate_hash(&self) -> String {
        let mut headers = self.clone();
        headers.hash = String::new(); // The hash is not part of the hash calculation.
        let serialized = serde_json::to_string(&headers).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(serialized.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }
}
