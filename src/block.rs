use chrono::Utc;
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use crate::fractal::FractalTriangle;

/// Represents a block in the SierpChain.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Block {
    pub index: u64,
    pub timestamp: i64,
    pub fractal: FractalTriangle,
    pub data: String,
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

// The expected time to mine a block, in seconds.
pub const BLOCK_GENERATION_INTERVAL: i64 = 10;
// The number of blocks after which to adjust the difficulty.
pub const DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = 10;


/// Represents the blockchain.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub difficulty: usize,
}

impl Blockchain {
    /// Creates a new blockchain with a genesis block.
    pub fn new(difficulty: usize) -> Self {
        let mut blockchain = Blockchain {
            chain: Vec::new(),
            difficulty,
        };
        blockchain.create_genesis_block();
        blockchain
    }

    /// Adjusts the mining difficulty based on the time it took to mine the last
    /// `DIFFICULTY_ADJUSTMENT_INTERVAL` blocks.
    ///
    /// The difficulty is adjusted to keep the block generation time close to
    /// `BLOCK_GENERATION_INTERVAL`.
    pub fn adjust_difficulty(&mut self) {
        let latest_block = self.chain.last().unwrap();
        if latest_block.index % DIFFICULTY_ADJUSTMENT_INTERVAL == 0 && latest_block.index != 0 {
            let previous_adjustment_block = &self.chain[(latest_block.index - DIFFICULTY_ADJUSTMENT_INTERVAL) as usize];
            let time_taken = latest_block.timestamp - previous_adjustment_block.timestamp;
            let expected_time = (DIFFICULTY_ADJUSTMENT_INTERVAL as i64) * BLOCK_GENERATION_INTERVAL;

            if time_taken < expected_time / 2 {
                self.difficulty += 1;
                println!("Difficulty increased to {}", self.difficulty);
            } else if time_taken > expected_time * 2 {
                if self.difficulty > 1 {
                    self.difficulty -= 1;
                    println!("Difficulty decreased to {}", self.difficulty);
                }
            }
        }
    }

    /// Creates the genesis block for the blockchain.
    fn create_genesis_block(&mut self) {
        let genesis_block = Block {
            index: 0,
            timestamp: Utc::now().timestamp(),
            fractal: FractalTriangle::generate(0),
            data: String::from("Genesis Block"),
            previous_hash: "0".to_string(),
            hash: String::new(),
            nonce: 0,
        };
        let mined_genesis = self.mine_block(genesis_block);
        self.chain.push(mined_genesis);
    }

    /// Mines a block using a proof-of-work algorithm.
    ///
    /// The algorithm requires finding a hash that starts with a certain number of zeros.
    pub fn mine_block(&self, mut block: Block) -> Block {
        let prefix = "0".repeat(self.difficulty);
        while !block.calculate_hash().starts_with(&prefix) {
            block.nonce += 1;
        }
        block.hash = block.calculate_hash();
        block
    }

    /// Adds a new block to the blockchain.
    pub fn add_block(&mut self, fractal_depth: usize, data: String) {
        let previous_block = self.chain.last().unwrap().clone();
        let new_block = Block {
            index: previous_block.index + 1,
            timestamp: Utc::now().timestamp(),
            fractal: FractalTriangle::generate(fractal_depth),
            data,
            previous_hash: previous_block.hash.clone(),
            hash: String::new(),
            nonce: 0,
        };
        let mined_block = self.mine_block(new_block);
        self.chain.push(mined_block);
        self.adjust_difficulty();
    }

    pub fn add_block_from_network(&mut self, block: Block) {
        let previous_block = self.chain.last().unwrap();
        if self.is_block_valid(&block, previous_block) {
            self.chain.push(block);
            self.adjust_difficulty();
        }
    }

    /// Validates a block.
    fn is_block_valid(&self, new_block: &Block, previous_block: &Block) -> bool {
        if new_block.index != previous_block.index + 1 {
            return false;
        }
        if new_block.previous_hash != previous_block.hash {
            return false;
        }
        let prefix = "0".repeat(self.difficulty);
        if !new_block.hash.starts_with(&prefix) || new_block.hash != new_block.calculate_hash() {
            return false;
        }
        // Timestamp validation
        let now = Utc::now().timestamp();
        if new_block.timestamp > now + 30 { // 30 seconds tolerance for future blocks
            return false;
        }
        if new_block.timestamp < previous_block.timestamp {
            return false;
        }
        true
    }
}
