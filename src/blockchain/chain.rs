use chrono::Utc;
use serde::{Serialize, Deserialize};
use std::collections::HashSet;
use std::fs;
use std::io::Write;

use super::block::Block;
use crate::core::fractal::FractalTriangle;
use crate::core::transaction::{Transaction, TxInput, TxOutput};
use crate::mining::miner::Miner;

const DB_FILE: &str = "blockchain.json";

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
    /// Creates a new blockchain, loading from a file if it exists.
    pub fn new(difficulty: usize) -> Self {
        if let Ok(file_content) = fs::read_to_string(DB_FILE) {
            if let Ok(mut blockchain) = serde_json::from_str::<Blockchain>(&file_content) {
                println!("Loaded blockchain from {}", DB_FILE);
                if blockchain.chain.is_empty() {
                    blockchain.create_genesis_block();
                }
                return blockchain;
            }
        }

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
        let coinbase_tx = Transaction::new(
            vec![TxInput {
                txid: "0".repeat(64),
                vout: usize::MAX,
                script_sig: String::from("genesis"),
                pub_key: String::new(),
                sequence: 0,
            }],
            vec![TxOutput {
                value: 50,
                script_pub_key: String::from("genesis_address"), // Placeholder
            }],
        );

        let genesis_block = Block {
            index: 0,
            timestamp: Utc::now().timestamp(),
            fractal: FractalTriangle { depth: 0, seed: 0, vertices: vec![] }, // Placeholder
            transactions: vec![coinbase_tx],
            previous_hash: "0".to_string(),
            hash: String::new(),
            nonce: 0,
        };
        let mined_genesis = Miner::mine_block(self.difficulty, 0, genesis_block);
        self.chain.push(mined_genesis);
    }

    /// Adds a new block to the blockchain and returns it.
    pub fn add_block(&mut self, fractal_depth: usize, transactions: Vec<Transaction>) -> Block {
        let previous_block = self.chain.last().unwrap().clone();
        let new_block = Block {
            index: previous_block.index + 1,
            timestamp: Utc::now().timestamp(),
            fractal: FractalTriangle { depth: fractal_depth, seed: 0, vertices: vec![] }, // Placeholder
            transactions,
            previous_hash: previous_block.hash.clone(),
            hash: String::new(),
            nonce: 0,
        };
        let mined_block = Miner::mine_block(self.difficulty, fractal_depth, new_block);
        self.chain.push(mined_block.clone());
        self.adjust_difficulty();
        mined_block
    }

    pub fn add_block_from_network(&mut self, block: Block) -> bool {
        let previous_block = self.chain.last().unwrap();
        if self.is_block_valid(&block, previous_block) {
            self.chain.push(block);
            self.adjust_difficulty();
            true
        } else {
            false
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

    /// Returns the UTXOs for a given address.
    pub fn get_utxos(&self, address: &str) -> Vec<(String, usize, TxOutput)> {
        let mut utxos = Vec::new();
        let mut spent_txos = HashSet::new();

        for block in &self.chain {
            for tx in &block.transactions {
                for input in &tx.inputs {
                    spent_txos.insert((input.txid.clone(), input.vout));
                }
            }
        }

        for block in &self.chain {
            for tx in &block.transactions {
                for (vout, output) in tx.outputs.iter().enumerate() {
                    if output.script_pub_key == address {
                        if !spent_txos.contains(&(tx.id.clone(), vout)) {
                            utxos.push((tx.id.clone(), vout, output.clone()));
                        }
                    }
                }
            }
        }

        utxos
    }

    /// Returns the balance for a given address.
    pub fn get_balance(&self, address: &str) -> u64 {
        self.get_utxos(address)
            .iter()
            .map(|(_, _, utxo)| utxo.value)
            .sum()
    }

    /// Saves the blockchain to a file.
    pub fn save_to_file(&self) -> std::io::Result<()> {
        let serialized = serde_json::to_string_pretty(&self).unwrap();
        let mut file = fs::File::create(DB_FILE)?;
        file.write_all(serialized.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::wallet::Wallet;

    #[test]
    fn test_get_balance_and_utxos() {
        let mut blockchain = Blockchain::new(1);
        let wallet1 = Wallet::new();
        let wallet2 = Wallet::new();

        // The genesis block creates a coinbase transaction. Let's assume it goes to a burn address for simplicity.
        // In our implementation, it goes to "genesis_address".

        let tx1 = Transaction::new(
            vec![], // No inputs, this is not a valid tx, but for testing balance it is ok
            vec![
                TxOutput {
                    value: 20,
                    script_pub_key: wallet1.get_address(),
                },
                TxOutput {
                    value: 30,
                    script_pub_key: wallet1.get_address(),
                },
            ],
        );

        let tx2 = Transaction::new(
            vec![TxInput {
                txid: tx1.id.clone(),
                vout: 0,
                script_sig: String::new(),
                pub_key: String::new(),
                sequence: 0,
            }],
            vec![TxOutput {
                value: 20,
                script_pub_key: wallet2.get_address(),
            }],
        );

        let _ = blockchain.add_block(1, vec![tx1]);
        let _ = blockchain.add_block(1, vec![tx2]);

        // Wallet 1 should have 30 (one output of 20 was spent)
        assert_eq!(blockchain.get_balance(&wallet1.get_address()), 30);
        // Wallet 2 should have 20
        assert_eq!(blockchain.get_balance(&wallet2.get_address()), 20);

        // Wallet 1 should have one UTXO
        assert_eq!(blockchain.get_utxos(&wallet1.get_address()).len(), 1);
        // Wallet 2 should have one UTXO
        assert_eq!(blockchain.get_utxos(&wallet2.get_address()).len(), 1);
    }
}
