mod fractal;
mod block;
mod p2p;
mod transaction;
mod wallet;

use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpServer, Responder, HttpResponse};
use block::Blockchain;
use p2p::P2pMessage;
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use transaction::{Transaction, TxInput, TxOutput};
use wallet::Wallet;
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

#[get("/address/{address}/balance")]
async fn get_balance(
    address: web::Path<String>,
    blockchain: web::Data<Arc<Mutex<Blockchain>>>,
) -> impl Responder {
    let blockchain = blockchain.lock().unwrap();
    let balance = blockchain.get_balance(&address.into_inner());
    web::Json(balance)
}

#[get("/address/{address}/utxos")]
async fn get_utxos(
    address: web::Path<String>,
    blockchain: web::Data<Arc<Mutex<Blockchain>>>,
) -> impl Responder {
    let blockchain = blockchain.lock().unwrap();
    let utxos = blockchain.get_utxos(&address.into_inner());
    web::Json(utxos)
}

#[get("/wallet/info")]
async fn get_wallet_info(
    blockchain: web::Data<Arc<Mutex<Blockchain>>>,
    miner_wallet: web::Data<Arc<Wallet>>,
) -> impl Responder {
    let address = miner_wallet.get_address();
    let balance = {
        let blockchain = blockchain.lock().unwrap();
        blockchain.get_balance(&address)
    };
    web::Json(serde_json::json!({
        "address": address,
        "balance": balance,
    }))
}

#[derive(Deserialize)]
struct TransactRequest {
    to: String,
    amount: u64,
}

#[post("/transact")]
async fn transact(
    req: web::Json<TransactRequest>,
    blockchain: web::Data<Arc<Mutex<Blockchain>>>,
    tx_pool: web::Data<TransactionPool>,
    p2p_sender: web::Data<mpsc::UnboundedSender<P2pMessage>>,
    miner_wallet: web::Data<Arc<Wallet>>,
) -> impl Responder {
    let blockchain = blockchain.lock().unwrap();
    let sender_address = miner_wallet.get_address();
    let utxos = blockchain.get_utxos(&sender_address);

    let mut inputs = vec![];
    let mut accumulated = 0;
    for (txid, vout, utxo) in utxos {
        inputs.push(TxInput {
            txid,
            vout,
            script_sig: String::new(),
            pub_key: String::new(),
            sequence: 0,
        });
        accumulated += utxo.value;
        if accumulated >= req.amount {
            break;
        }
    }

    if accumulated < req.amount {
        return HttpResponse::BadRequest().body("Not enough funds");
    }

    let mut outputs = vec![TxOutput {
        value: req.amount,
        script_pub_key: req.to.clone(),
    }];

    if accumulated > req.amount {
        outputs.push(TxOutput {
            value: accumulated - req.amount,
            script_pub_key: sender_address,
        });
    }

    let mut new_tx = Transaction::new(inputs, outputs);
    new_tx.sign(&miner_wallet);

    if !new_tx.verify() {
        return HttpResponse::InternalServerError().body("Failed to verify new transaction");
    }

    p2p_sender.send(P2pMessage::Transaction(new_tx.clone())).unwrap();

    let mut pool = tx_pool.lock().unwrap();
    pool.push(new_tx.clone());

    HttpResponse::Ok().json(new_tx)
}

type TransactionPool = Arc<Mutex<Vec<Transaction>>>;

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
    let transaction_pool: TransactionPool = Arc::new(Mutex::new(vec![]));
    let miner_wallet = Arc::new(Wallet::new());

    println!("Genesis block mined: {:#?}", blockchain.lock().unwrap().chain.first().unwrap());
    println!("Miner address: {}", miner_wallet.get_address());


    let p2p = p2p::P2p::new(p2p_message_sender, to_p2p_receiver).await;
    tokio::spawn(p2p.run());

    // Spawn a new thread for mining blocks.
    let blockchain_for_mining = Arc::clone(&blockchain);
    let transaction_pool_for_mining = Arc::clone(&transaction_pool);
    let to_p2p_sender_for_mining = to_p2p_sender.clone();
    let miner_wallet_for_mining = Arc::clone(&miner_wallet);
    tokio::spawn(async move {
        loop {
            // Wait for some time before mining the next block.
            time::sleep(Duration::from_secs(10)).await;

            let new_block;
            {
                let mut transactions = transaction_pool_for_mining.lock().unwrap();
                let mut blockchain_lock = blockchain_for_mining.lock().unwrap();

                // Create a coinbase transaction to reward the miner.
                let coinbase_tx = Transaction::new(
                    vec![TxInput {
                        txid: "0".repeat(64),
                        vout: blockchain_lock.chain.len() as usize,
                        script_sig: String::from("coinbase"),
                        pub_key: String::new(),
                        sequence: 0,
                    }],
                    vec![TxOutput {
                        value: 50, // Reward
                        script_pub_key: miner_wallet_for_mining.get_address(),
                    }],
                );

                let mut block_transactions = vec![coinbase_tx];
                block_transactions.extend(transactions.drain(..));


                println!("\nMining block {}...", blockchain_lock.chain.len());
                blockchain_lock.add_block(5, block_transactions);
                new_block = blockchain_lock.chain.last().unwrap().clone();
                to_p2p_sender_for_mining.send(P2pMessage::Block(new_block.clone())).unwrap();

                if let Err(e) = blockchain_lock.save_to_file() {
                    tracing::error!("Failed to save blockchain: {}", e);
                }
            } // Mutex lock is released here.

            println!("Block {} mined: {:#?}", new_block.index, new_block);
        }
    });

    let blockchain_for_networking = Arc::clone(&blockchain);
    let transaction_pool_for_networking = Arc::clone(&transaction_pool);
    let to_p2p_sender_for_networking = to_p2p_sender.clone();
    tokio::spawn(async move {
        while let Some(message) = p2p_message_receiver.recv().await {
            match message {
                P2pMessage::Block(block) => {
                    let mut blockchain_lock = blockchain_for_networking.lock().unwrap();
                    blockchain_lock.add_block_from_network(block);
                    if let Err(e) = blockchain_lock.save_to_file() {
                        tracing::error!("Failed to save blockchain: {}", e);
                    }
                }
                P2pMessage::ChainRequest => {
                    let blockchain_lock = blockchain_for_networking.lock().unwrap();
                    let chain = blockchain_lock.clone();
                    to_p2p_sender_for_networking.send(P2pMessage::ChainResponse(chain)).unwrap();
                }
                P2pMessage::ChainResponse(chain) => {
                    let mut blockchain_lock = blockchain_for_networking.lock().unwrap();
                    if chain.chain.len() > blockchain_lock.chain.len() {
                        // Basic validation
                        // In a real application, you'd want to do a full validation
                        // of the chain.
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
                            println!("Received valid transaction, adding to pool: {}", transaction.id);
                            pool.push(transaction);
                        }
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
            .app_data(web::Data::new(Arc::clone(&transaction_pool)))
            .app_data(web::Data::new(to_p2p_sender.clone()))
            .app_data(web::Data::new(Arc::clone(&miner_wallet)))
            .service(get_blocks)
            .service(get_balance)
            .service(get_utxos)
            .service(transact)
            .service(get_wallet_info)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}