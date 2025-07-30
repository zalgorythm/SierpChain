use chrono::Utc;
use sha2::{Sha256, Digest};
use serde::{Serialize};
use crate::fractal::FractalTriangle;

#[derive(Serialize, Debug, Clone)]
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
    pub fn calculate_hash(&self) -> String {
        let mut headers = self.clone();
        headers.hash = String::new();
        let serialized = serde_json::to_string(&headers).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(serialized.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }
}

pub struct Blockchain {
    pub chain: Vec<Block>,
    pub difficulty: usize,
}

impl Blockchain {
    pub fn new(difficulty: usize) -> Self {
        let mut blockchain = Blockchain {
            chain: Vec::new(),
            difficulty,
        };
        blockchain.create_genesis_block();
        blockchain
    }

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
        let hash = genesis_block.calculate_hash();
        let mut mined_genesis = self.mine_block(genesis_block);
        mined_genesis.hash = mined_genesis.calculate_hash();
        self.chain.push(mined_genesis);
    }

    pub fn mine_block(&self, mut block: Block) -> Block {
        let prefix = "0".repeat(self.difficulty);
        while !block.calculate_hash().starts_with(&prefix) {
            block.nonce += 1;
        }
        block.hash = block.calculate_hash();
        block
    }

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
    }
}
