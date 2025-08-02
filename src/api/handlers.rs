use actix_web::{get, post, web, Responder, HttpResponse};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use crate::blockchain::chain::Blockchain;
use crate::core::transaction::{Transaction, TxInput, TxOutput};
use crate::core::wallet::Wallet;
use crate::network::p2p::P2pMessage;
use crate::fractal::FractalType;
use ed25519_dalek::SigningKey;
use hex;

pub type TransactionPool = Arc<Mutex<Vec<Transaction>>>;

#[derive(Deserialize, Debug)]
#[serde(tag = "type", content = "params")]
pub enum MineRequestParams {
    Sierpinski {
        depth: usize,
    },
    Mandelbrot {
        width: usize,
        height: usize,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        max_iterations: u32,
    },
    Julia {
        width: usize,
        height: usize,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        c_real: f64,
        c_imag: f64,
        max_iterations: u32,
    },
}

impl MineRequestParams {
    // This function will be used to convert the request params to the internal FractalType
    // The seed will be set to 0, as it will be determined by the miner.
    pub fn to_fractal_type(&self) -> FractalType {
        match self {
            MineRequestParams::Sierpinski { depth } => FractalType::Sierpinski { depth: *depth, seed: 0 },
            MineRequestParams::Mandelbrot { width, height, x_min, x_max, y_min, y_max, max_iterations } => {
                FractalType::Mandelbrot {
                    width: *width,
                    height: *height,
                    x_min: *x_min,
                    x_max: *x_max,
                    y_min: *y_min,
                    y_max: *y_max,
                    max_iterations: *max_iterations,
                    seed: 0,
                }
            }
            MineRequestParams::Julia { width, height, x_min, x_max, y_min, y_max, c_real, c_imag, max_iterations } => {
                FractalType::Julia {
                    width: *width,
                    height: *height,
                    x_min: *x_min,
                    x_max: *x_max,
                    y_min: *y_min,
                    y_max: *y_max,
                    c_real: *c_real,
                    c_imag: *c_imag,
                    max_iterations: *max_iterations,
                    seed: 0,
                }
            }
        }
    }
}


#[post("/mine")]
pub async fn mine(
    blockchain: web::Data<Arc<Mutex<Blockchain>>>,
    transaction_pool: web::Data<TransactionPool>,
    to_p2p: web::Data<mpsc::UnboundedSender<P2pMessage>>,
    miner_wallet: web::Data<Arc<Wallet>>,
    params: Option<web::Json<MineRequestParams>>,
) -> impl Responder {
    let mut blockchain = blockchain.lock().unwrap();
    let mut transactions = transaction_pool.lock().unwrap();

    let coinbase_tx = Transaction::new(
        vec![TxInput {
            txid: "0".repeat(64),
            vout: blockchain.chain.len() as usize,
            script_sig: String::from("coinbase"),
            pub_key: String::new(),
            sequence: 0,
        }],
        vec![TxOutput {
            value: 50, // Reward
            script_pub_key: miner_wallet.get_address(),
        }],
    );

    let mut block_transactions = vec![coinbase_tx];
    block_transactions.extend(transactions.drain(..));

    let fractal_type = params.map_or_else(
        || FractalType::Sierpinski { depth: 5, seed: 0 }, // Default
        |p| p.into_inner().to_fractal_type(),
    );

    let mined_block = blockchain.add_block(fractal_type, block_transactions);

    if let Err(e) = blockchain.save_to_file() {
        tracing::error!("Failed to save blockchain: {}", e);
    }

    to_p2p.send(P2pMessage::Block(mined_block.clone())).unwrap();

    HttpResponse::Ok().json(mined_block)
}

#[get("/blocks")]
pub async fn get_blocks(data: web::Data<Arc<Mutex<Blockchain>>>) -> impl Responder {
    let blockchain = data.lock().unwrap();
    web::Json(blockchain.chain.clone())
}

#[get("/address/{address}/balance")]
pub async fn get_balance(
    address: web::Path<String>,
    blockchain: web::Data<Arc<Mutex<Blockchain>>>,
) -> impl Responder {
    let blockchain = blockchain.lock().unwrap();
    let balance = blockchain.get_balance(&address.into_inner());
    web::Json(balance)
}

#[get("/address/{address}/utxos")]
pub async fn get_utxos(
    address: web::Path<String>,
    blockchain: web::Data<Arc<Mutex<Blockchain>>>,
) -> impl Responder {
    let blockchain = blockchain.lock().unwrap();
    let utxos = blockchain.get_utxos(&address.into_inner());
    web::Json(utxos)
}

#[get("/wallet/info")]
pub async fn get_wallet_info(
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
pub struct TransactRequest {
    to: String,
    amount: u64,
    private_key: String,
}

#[post("/transact")]
pub async fn transact(
    req: web::Json<TransactRequest>,
    blockchain: web::Data<Arc<Mutex<Blockchain>>>,
    tx_pool: web::Data<TransactionPool>,
    p2p_sender: web::Data<mpsc::UnboundedSender<P2pMessage>>,
) -> impl Responder {
    let private_key_bytes = match hex::decode(&req.private_key) {
        Ok(bytes) => bytes,
        Err(_) => return HttpResponse::BadRequest().body("Invalid private key format"),
    };

    let private_key_array: [u8; 32] = match private_key_bytes.try_into() {
        Ok(arr) => arr,
        Err(_) => return HttpResponse::BadRequest().body("Invalid private key length"),
    };

    let signing_key = SigningKey::from_bytes(&private_key_array);

    let sender_wallet = Wallet { signing_key };
    let sender_address = sender_wallet.get_address();

    let blockchain = blockchain.lock().unwrap();
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
    new_tx.sign(&sender_wallet);

    if !new_tx.verify() {
        return HttpResponse::InternalServerError().body("Failed to verify new transaction");
    }

    p2p_sender.send(P2pMessage::Transaction(new_tx.clone())).unwrap();

    let mut pool = tx_pool.lock().unwrap();
    pool.push(new_tx.clone());

    HttpResponse::Ok().json(new_tx)
}

#[derive(Serialize)]
struct WalletInfoResponse {
    private_key: String,
    public_key: String,
    address: String,
}

#[post("/wallet")]
pub async fn create_wallet() -> impl Responder {
    let wallet = Wallet::new();
    let response = WalletInfoResponse {
        private_key: hex::encode(wallet.signing_key.to_bytes()),
        public_key: hex::encode(wallet.get_public_key().as_bytes()),
        address: wallet.get_address(),
    };
    HttpResponse::Ok().json(response)
}
