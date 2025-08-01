use actix_web::{get, post, web, Responder, HttpResponse};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use crate::blockchain::block::Blockchain;
use crate::core::transaction::{Transaction, TxInput, TxOutput};
use crate::core::wallet::Wallet;
use crate::network::p2p::P2pMessage;

pub type TransactionPool = Arc<Mutex<Vec<Transaction>>>;

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
}

#[post("/transact")]
pub async fn transact(
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
