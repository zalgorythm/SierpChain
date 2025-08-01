// Declare the top-level modules
mod api;
mod blockchain;
mod core;
mod network;

use crate::api::handlers::{
    get_blocks, get_balance, get_utxos, transact, get_wallet_info, TransactionPool,
};
use crate::api::websocket::{BroadcastBlock, BroadcastHub, WsConn};
use crate::blockchain::block::{Block, Blockchain};
use crate::core::transaction::{Transaction, TxInput, TxOutput};
use crate::core::wallet::Wallet;
use crate::network::p2p::{P2p, P2pMessage};

use actix::{Actor, Addr};
use actix_cors::Cors;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Error};
use actix_web_actors::ws;
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::time::{self, Duration};
use tracing_subscriber::fmt;

// Initialize the tracing subscriber.
static TRACING_SUBSCRIBER: Lazy<()> = Lazy::new(|| {
    fmt::init();
});

/// WebSocket handshake and actor starting
async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    hub_addr: web::Data<Addr<BroadcastHub>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        WsConn::new(hub_addr.get_ref().clone()),
        &req,
        stream,
    )
}

/// The main entry point for the SierpChain backend.
#[tokio::main]
async fn main() -> std::io::Result<()> {
    Lazy::force(&TRACING_SUBSCRIBER);

    // Start the broadcast hub
    let hub = BroadcastHub::new().start();

    // Create channels for P2P communication.
    let (p2p_message_sender, mut p2p_message_receiver) = mpsc::unbounded_channel::<P2pMessage>();
    let (to_p2p_sender, to_p2p_receiver) = mpsc::unbounded_channel::<P2pMessage>();

    // Initialize shared state.
    let blockchain = Arc::new(Mutex::new(Blockchain::new(4)));
    let transaction_pool: TransactionPool = Arc::new(Mutex::new(vec![]));
    let miner_wallet = Arc::new(Wallet::new());

    println!(
        "Genesis block mined: {:#?}",
        blockchain.lock().unwrap().chain.first().unwrap()
    );
    println!("Miner address: {}", miner_wallet.get_address());

    // Start the P2P network layer.
    let p2p = P2p::new(p2p_message_sender, to_p2p_receiver).await;
    tokio::spawn(p2p.run());

    // Spawn a new thread for mining blocks.
    let blockchain_for_mining = Arc::clone(&blockchain);
    let transaction_pool_for_mining = Arc::clone(&transaction_pool);
    let to_p2p_sender_for_mining = to_p2p_sender.clone();
    let miner_wallet_for_mining = Arc::clone(&miner_wallet);
    let hub_for_mining = hub.clone();
    tokio::spawn(async move {
        loop {
            time::sleep(Duration::from_secs(10)).await;

            let new_block: Block;
            {
                let mut transactions = transaction_pool_for_mining.lock().unwrap();
                let mut blockchain_lock = blockchain_for_mining.lock().unwrap();

                let coinbase_tx = Transaction::new(
                    vec![TxInput {
                        txid: "0".repeat(64),
                        vout: blockchain_lock.chain.len() as usize,
                        script_sig: String::from("coinbase"),
                        pub_key: String::new(),
                        sequence: 0,
                    }],
                    vec![TxOutput {
                        value: 50,
                        script_pub_key: miner_wallet_for_mining.get_address(),
                    }],
                );

                let mut block_transactions = vec![coinbase_tx];
                block_transactions.extend(transactions.drain(..));

                println!("\nMining block {}...", blockchain_lock.chain.len());
                blockchain_lock.add_block(5, block_transactions);
                new_block = blockchain_lock.chain.last().unwrap().clone();
                to_p2p_sender_for_mining
                    .send(P2pMessage::Block(new_block.clone()))
                    .unwrap();

                // Broadcast the new block to WebSocket clients
                hub_for_mining.do_send(BroadcastBlock { block: new_block.clone() });

                if let Err(e) = blockchain_lock.save_to_file() {
                    tracing::error!("Failed to save blockchain: {}", e);
                }
            }
            println!("Block {} mined: {:#?}", new_block.index, new_block);
        }
    });

    // Spawn a thread to handle incoming P2P messages.
    let blockchain_for_networking = Arc::clone(&blockchain);
    let transaction_pool_for_networking = Arc::clone(&transaction_pool);
    let to_p2p_sender_for_networking = to_p2p_sender.clone();
    let hub_for_networking = hub.clone();
    tokio::spawn(async move {
        while let Some(message) = p2p_message_receiver.recv().await {
            match message {
                P2pMessage::Block(block) => {
                    let mut blockchain_lock = blockchain_for_networking.lock().unwrap();
                    let added = blockchain_lock.add_block_from_network(block.clone());
                    if added {
                        hub_for_networking.do_send(BroadcastBlock { block });
                    }
                    if let Err(e) = blockchain_lock.save_to_file() {
                        tracing::error!("Failed to save blockchain: {}", e);
                    }
                }
                P2pMessage::ChainRequest => {
                    let blockchain_lock = blockchain_for_networking.lock().unwrap();
                    let chain = blockchain_lock.clone();
                    to_p2p_sender_for_networking
                        .send(P2pMessage::ChainResponse(chain))
                        .unwrap();
                }
                P2pMessage::ChainResponse(chain) => {
                    let mut blockchain_lock = blockchain_for_networking.lock().unwrap();
                    if chain.chain.len() > blockchain_lock.chain.len() {
                        blockchain_lock.chain = chain.chain;
                        if let Err(e) = blockchain_lock.save_to_file() {
                            tracing::error!("Failed to save blockchain: {}", e);
                        }
                    }
                }
                P2pMessage::Transaction(transaction) => {
                    if transaction.verify() {
                        let mut pool = transaction_pool_for_networking.lock().unwrap();
                        if !pool.iter().any(|tx| tx.id == transaction.id) {
                            pool.push(transaction);
                        }
                    }
                }
            }
        }
    });

    println!("Starting web server at http://127.0.0.1:8080");
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(Arc::clone(&blockchain)))
            .app_data(web::Data::new(Arc::clone(&transaction_pool)))
            .app_data(web::Data::new(to_p2p_sender.clone()))
            .app_data(web::Data::new(Arc::clone(&miner_wallet)))
            .app_data(web::Data::new(hub.clone()))
            .service(get_blocks)
            .service(get_balance)
            .service(get_utxos)
            .service(transact)
            .service(get_wallet_info)
            .route("/ws", web::get().to(ws_route))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}