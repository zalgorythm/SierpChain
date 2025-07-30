mod fractal;
mod block;

use block::Blockchain;
use std::{thread, time};

fn main() {
    let mut blockchain = Blockchain::new(4);
    println!("Mining genesis block...");
    println!("Genesis block mined: {:#?}", blockchain.chain.first().unwrap());

    let mut block_counter = 1;
    loop {
        println!("\nMining block {}...", block_counter);
        let data = format!("Block {} data", block_counter);
        blockchain.add_block(5, data);
        println!("Block {} mined: {:#?}", block_counter, blockchain.chain.last().unwrap());
        block_counter += 1;
        thread::sleep(time::Duration::from_secs(1));
    }
}