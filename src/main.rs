// Declare the top-level modules
mod api;
mod blockchain;
mod core;
mod fractal;
mod network;
mod mining;

use crate::api::handlers::{
    get_blocks, get_balance, get_utxos, transact, get_wallet_info, mine, create_wallet, TransactionPool,
};
use crate::api::websocket::{BroadcastBlock, BroadcastHub, WsConn};
use crate::blockchain::chain::Blockchain;
use crate::core::wallet::Wallet;
use crate::network::p2p::{P2p, P2pMessage};

use actix::{Actor, Addr};
use actix_cors::Cors;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Error};
use actix_web_actors::ws;
use clap::Parser;
use dotenv::dotenv;
use libp2p::Multiaddr;
use once_cell::sync::Lazy;
use std::env;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::time::{self, Duration};
use tracing_subscriber::fmt;

// Initialize the tracing subscriber.
static TRACING_SUBSCRIBER: Lazy<()> = Lazy::new(|| {
    fmt::init();
});

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value_t = 8080)]
    http_port: u16,
    #[arg(short, long, default_value_t = 0)]
    p2p_port: u16,
    #[arg(long)]
    peer: Vec<Multiaddr>,
}

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
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    Lazy::force(&TRACING_SUBSCRIBER);
    dotenv().ok();
    let cli = Cli::parse();

    // Start the broadcast hub
    let hub = BroadcastHub::new().start();

    // Create channels for P2P communication.
    let (p2p_message_sender, mut p2p_message_receiver) = mpsc::unbounded_channel::<P2pMessage>();
    let (to_p2p_sender, to_p2p_receiver) = mpsc::unbounded_channel::<P2pMessage>();

    // Initialize shared state.
    let blockchain = Arc::new(Mutex::new(Blockchain::new(2)));
    let transaction_pool: TransactionPool = Arc::new(Mutex::new(vec![]));
    let miner_wallet = Arc::new(Wallet::new());

    println!(
        "Genesis block mined: {:#?}",
        blockchain.lock().unwrap().chain.first().unwrap()
    );
    println!("Miner address: {}", miner_wallet.get_address());

    // Start the P2P network layer.
    let p2p = P2p::new(p2p_message_sender, to_p2p_receiver, cli.p2p_port, cli.peer).await;
    tokio::spawn(p2p.run());

    // Spawn a thread to handle incoming P2P messages.
    let blockchain_for_networking = Arc::clone(&blockchain);
    let transaction_pool_for_networking = Arc::clone(&transaction_pool);
    let to_p2p_sender_for_networking = to_p2p_sender.clone();
    let hub_for_networking = hub.clone();
    let miner_wallet_for_networking = Arc::clone(&miner_wallet);
    tokio::spawn(async move {
        let auto_mine = env::var("AUTO_MINE").unwrap_or_else(|_| "false".to_string()) == "true";
        let interval_ms = env::var("MINING_INTERVAL_MS")
            .unwrap_or_else(|_| "10000".to_string())
            .parse::<u64>()
            .unwrap_or(10000);
        let mut mine_interval = if auto_mine {
            let mut interval = time::interval(Duration::from_millis(interval_ms));
            interval.tick().await; // First tick is immediate
            Some(interval)
        } else {
            None
        };

        loop {
            tokio::select! {
                Some(message) = p2p_message_receiver.recv() => {
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
                _ = async {
                    if let Some(ref mut interval) = mine_interval {
                        interval.tick().await;
                    } else {
                        // This branch will never be selected if auto-mining is disabled
                        std::future::pending::<()>().await;
                    }
                } => {
                    let mined_block = {
                        let mut blockchain = blockchain_for_networking.lock().unwrap();
                        let mut transactions = transaction_pool_for_networking.lock().unwrap();
                        let coinbase_tx = crate::core::transaction::Transaction::new(
                            vec![crate::core::transaction::TxInput {
                                txid: "0".repeat(64),
                                vout: blockchain.chain.len() as usize,
                                script_sig: String::from("coinbase"),
                                pub_key: String::new(),
                                sequence: 0,
                            }],
                            vec![crate::core::transaction::TxOutput {
                                value: 50,
                                script_pub_key: miner_wallet_for_networking.get_address(),
                            }],
                        );
                        let mut block_transactions = vec![coinbase_tx];
                        block_transactions.extend(transactions.drain(..));
                        let fractal_type = crate::fractal::FractalType::Sierpinski { depth: 5, seed: 0 };
                        let mined_block = blockchain.add_block(fractal_type, block_transactions);
                        if let Err(e) = blockchain.save_to_file() {
                            tracing::error!("Failed to save blockchain: {}", e);
                        }
                        mined_block
                    };
                    hub_for_networking.do_send(BroadcastBlock { block: mined_block.clone() });
                    to_p2p_sender_for_networking.send(P2pMessage::Block(mined_block)).unwrap();
                }
            }
        }
    });

    let http_addr = format!("127.0.0.1:{}", cli.http_port);
    println!("Starting web server at http://{}", http_addr);
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
            .service(mine)
            .service(create_wallet)
            .route("/ws", web::get().to(ws_route))
    })
    .bind(http_addr)?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App, dev::{Service, ServiceResponse}};
    use actix_http::Request;
    use serde_json;
    use hex;

    async fn setup_test_app() -> (impl Service<Request, Response = ServiceResponse, Error = actix_web::Error>, String) {
        std::fs::remove_file("blockchain.json").ok();
        let blockchain = Arc::new(Mutex::new(Blockchain::new(1)));
        let transaction_pool: TransactionPool = Arc::new(Mutex::new(vec![]));
        let miner_wallet = Arc::new(Wallet::new());
        let private_key = hex::encode(miner_wallet.signing_key.to_bytes());
        let (p2p_sender, mut p2p_receiver) = mpsc::unbounded_channel::<P2pMessage>();
        tokio::spawn(async move {
            while let Some(_) = p2p_receiver.recv().await {}
        });
        let hub = BroadcastHub::new().start();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::clone(&blockchain)))
                .app_data(web::Data::new(Arc::clone(&transaction_pool)))
                .app_data(web::Data::new(p2p_sender.clone()))
                .app_data(web::Data::new(Arc::clone(&miner_wallet)))
                .app_data(web::Data::new(hub.clone()))
                .service(api::handlers::create_wallet)
                .service(api::handlers::get_blocks)
                .service(api::handlers::mine)
                .service(api::handlers::transact)
                .service(api::handlers::get_wallet_info)
                .service(api::handlers::get_balance)
                .service(api::handlers::get_utxos)
                .route("/ws", web::get().to(ws_route))
        ).await;
        (app, private_key)
    }

    #[actix_web::test]
    async fn test_create_wallet_endpoint() {
        let (app, _) = setup_test_app().await;
        let req = test::TestRequest::post().uri("/wallet").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body["private_key"].is_string());
        assert!(body["public_key"].is_string());
        assert!(body["address"].is_string());
    }

    #[actix_web::test]
    async fn test_mine_endpoint() {
        let (app, _) = setup_test_app().await;
        let req = test::TestRequest::post().uri("/mine").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["index"], 1);
        assert!(body["transactions"].as_array().unwrap().len() >= 1); // Coinbase tx
        assert_eq!(body["fractal"]["type"], "Sierpinski");
    }

    #[actix_web::test]
    async fn test_mine_mandelbrot_endpoint() {
        let (app, _) = setup_test_app().await;
        let mine_req = serde_json::json!({
            "type": "Mandelbrot",
            "params": {
                "width": 10,
                "height": 10,
                "x_min": -2.0,
                "x_max": 1.0,
                "y_min": -1.5,
                "y_max": 1.5,
                "max_iterations": 100
            }
        });
        let req = test::TestRequest::post().uri("/mine").set_json(&mine_req).to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["index"], 1);
        assert_eq!(body["fractal"]["type"], "Mandelbrot");
        assert_eq!(body["fractal"]["data"]["width"], 10);
    }

    #[actix_web::test]
    async fn test_transact_endpoint() {
        let (app, miner_private_key) = setup_test_app().await;

        // 1. Create a receiver wallet
        let req = test::TestRequest::post().uri("/wallet").to_request();
        let resp = test::call_service(&app, req).await;
        let receiver_wallet: serde_json::Value = test::read_body_json(resp).await;
        let receiver_address = receiver_wallet["address"].as_str().unwrap().to_string();

        // 2. Mine a block to give the miner_wallet some funds.
        let req = test::TestRequest::post().uri("/mine").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        // 3. Create a transaction from the miner to the receiver
        let transact_req = serde_json::json!({
            "to": receiver_address,
            "amount": 10,
            "private_key": miner_private_key
        });
        let req = test::TestRequest::post().uri("/transact").set_json(&transact_req).to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        // 4. Mine another block to include the transaction
        let req = test::TestRequest::post().uri("/mine").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        // 5. Check the receiver's balance
        let req = test::TestRequest::get().uri(&format!("/address/{}/balance", receiver_wallet["address"].as_str().unwrap())).to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let balance: u64 = test::read_body_json(resp).await;
        assert_eq!(balance, 10);
    }
}