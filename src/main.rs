mod fractal;
mod block;

use actix_cors::Cors;
use actix_web::{get, web, App, HttpServer, Responder};
use block::Blockchain;
use std::sync::{Arc, Mutex};
use std::{thread, time};

/// Handles the `GET /blocks` endpoint.
///
/// This function retrieves the current blockchain and returns it as a JSON response.
#[get("/blocks")]
async fn get_blocks(data: web::Data<Arc<Mutex<Blockchain>>>) -> impl Responder {
    let blockchain = data.lock().unwrap();
    web::Json(blockchain.chain.clone())
}

/// The main entry point for the SierpChain backend.
///
/// This function initializes the blockchain, starts a mining thread, and launches an
/// `actix-web` server to expose the blockchain via a REST API.
#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Create a new blockchain with a difficulty of 4.
    let blockchain = Arc::new(Mutex::new(Blockchain::new(4)));
    println!("Genesis block mined: {:#?}", blockchain.lock().unwrap().chain.first().unwrap());

    // Spawn a new thread for mining blocks.
    let blockchain_for_mining = Arc::clone(&blockchain);
    thread::spawn(move || {
        let mut block_counter = 1;
        loop {
            let data = format!("Block {} data", block_counter);
            let new_block;
            {
                // Lock the blockchain to add a new block.
                let mut blockchain_lock = blockchain_for_mining.lock().unwrap();
                println!("\nMining block {}...", blockchain_lock.chain.len());
                blockchain_lock.add_block(5, data);
                new_block = blockchain_lock.chain.last().unwrap().clone();
            } // Mutex lock is released here.

            println!("Block {} mined: {:#?}", block_counter, new_block);
            block_counter += 1;

            // Wait for 1 second before mining the next block.
            thread::sleep(time::Duration::from_secs(1));
        }
    });

    println!("Starting web server at http://127.0.0.1:8080");
    // Start the `actix-web` server.
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(Arc::clone(&blockchain)))
            .service(get_blocks)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}