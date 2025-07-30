mod fractal;
mod block;
mod p2p;

use actix_cors::Cors;
use actix_web::{get, web, App, HttpServer, Responder};
use block::{Block, Blockchain};
use p2p::P2pMessage;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::time::{self, Duration};
use once_cell::sync::Lazy;
use tracing_subscriber::fmt;


static TRACING_SUBSCRIBER: Lazy<()> = Lazy::new(|| {
    fmt::init();
});


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
    Lazy::force(&TRACING_SUBSCRIBER);

    let (p2p_message_sender, mut p2p_message_receiver) = mpsc::unbounded_channel::<P2pMessage>();
    let (to_p2p_sender, to_p2p_receiver) = mpsc::unbounded_channel::<P2pMessage>();


    // Create a new blockchain with a difficulty of 4.
    let blockchain = Arc::new(Mutex::new(Blockchain::new(4)));
    println!("Genesis block mined: {:#?}", blockchain.lock().unwrap().chain.first().unwrap());

    let p2p = p2p::P2p::new(p2p_message_sender, to_p2p_receiver).await;
    tokio::spawn(p2p.run());

    // Spawn a new thread for mining blocks.
    let blockchain_for_mining = Arc::clone(&blockchain);
    let to_p2p_sender_for_mining = to_p2p_sender.clone();
    tokio::spawn(async move {
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
                to_p2p_sender_for_mining.send(P2pMessage::Block(new_block.clone())).unwrap();
            } // Mutex lock is released here.

            println!("Block {} mined: {:#?}", block_counter, new_block);
            block_counter += 1;

            // Wait for 1 second before mining the next block.
            time::sleep(Duration::from_secs(1)).await;
        }
    });

    let blockchain_for_networking = Arc::clone(&blockchain);
    tokio::spawn(async move {
        while let Some(message) = p2p_message_receiver.recv().await {
            let mut blockchain_lock = blockchain_for_networking.lock().unwrap();
            match message {
                P2pMessage::Block(block) => {
                    blockchain_lock.add_block_from_network(block);
                }
                P2pMessage::ChainRequest => {
                    let chain = blockchain_lock.clone();
                    to_p2p_sender.send(P2pMessage::ChainResponse(chain)).unwrap();
                }
                P2pMessage::ChainResponse(chain) => {
                    if chain.chain.len() > blockchain_lock.chain.len() {
                        // Basic validation
                        // In a real application, you'd want to do a full validation
                        // of the chain.
                        blockchain_lock.chain = chain.chain;
                    }
                }
            }
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