use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use chrono::Utc;
use crate::core::wallet::Wallet;
use ed25519_dalek::{Signature, VerifyingKey, Verifier};

/// An input to a transaction.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TxInput {
    /// The ID of the transaction that this input is spending from.
    pub txid: String,
    /// The index of the output in the transaction that this input is spending from.
    pub vout: usize,
    /// The script that proves ownership of the output being spent.
    /// For now, this will just be the signature.
    pub script_sig: String,
    /// The public key of the sender.
    pub pub_key: String,
    /// The sequence number. Not used in this implementation.
    pub sequence: u32,
}

/// An output from a transaction.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TxOutput {
    /// The value of the output in the smallest unit of the currency.
    pub value: u64,
    /// The script that locks the output.
    /// For now, this will just be the recipient's public key hash.
    pub script_pub_key: String,
}

/// A transaction.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Transaction {
    /// The transaction ID.
    pub id: String,
    /// The time the transaction was created.
    pub timestamp: i64,
    /// The inputs to the transaction.
    pub inputs: Vec<TxInput>,
    /// The outputs from the transaction.
    pub outputs: Vec<TxOutput>,
}

impl Transaction {
    /// Creates a new transaction.
    pub fn new(inputs: Vec<TxInput>, outputs: Vec<TxOutput>) -> Self {
        let mut tx = Transaction {
            id: String::new(),
            timestamp: Utc::now().timestamp(),
            inputs,
            outputs,
        };
        tx.id = tx.calculate_hash();
        tx
    }

    /// Calculates the SHA-256 hash of the transaction.
    pub fn calculate_hash(&self) -> String {
        let mut tx_clone = self.clone();
        tx_clone.id = String::new(); // The id is not part of the hash calculation.
        // For signing and verification, we don't want to include the signature
        // in the hash.
        for input in &mut tx_clone.inputs {
            input.script_sig = String::new();
            input.pub_key = String::new();
        }

        let serialized = serde_json::to_string(&tx_clone).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(serialized.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    /// Signs the transaction with the provided wallet.
    /// This is a simplified signing method that assumes the wallet owns all inputs.
    pub fn sign(&mut self, wallet: &Wallet) {
        let tx_hash = self.calculate_hash();
        let signature = wallet.sign(tx_hash.as_bytes());

        for input in &mut self.inputs {
            input.script_sig = hex::encode(signature.to_bytes());
            input.pub_key = hex::encode(wallet.get_public_key().as_bytes());
        }
    }

    /// Verifies the transaction's signatures.
    /// This is a simplified verification method.
    pub fn verify(&self) -> bool {
        let tx_hash = self.calculate_hash();
        for input in &self.inputs {
            // Coinbase transactions have no real signature to verify
            if input.txid == "0".repeat(64) {
                continue;
            }

            let signature_bytes = match hex::decode(&input.script_sig) {
                Ok(bytes) => bytes,
                Err(_) => return false,
            };
            let signature_array: [u8; 64] = match signature_bytes.try_into() {
                Ok(arr) => arr,
                Err(_) => return false,
            };
            let signature = Signature::from_bytes(&signature_array);

            let pub_key_bytes = match hex::decode(&input.pub_key) {
                Ok(bytes) => bytes,
                Err(_) => return false,
            };
            let pub_key_array: [u8; 32] = match pub_key_bytes.try_into() {
                Ok(arr) => arr,
                Err(_) => return false,
            };
            let verifying_key = match VerifyingKey::from_bytes(&pub_key_array) {
                Ok(key) => key,
                Err(_) => return false,
            };

            if verifying_key.verify(tx_hash.as_bytes(), &signature).is_err() {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::wallet::Wallet;

    #[test]
    fn test_sign_and_verify_transaction() {
        let wallet = Wallet::new();
        let mut tx = Transaction::new(
            vec![TxInput {
                txid: "0".repeat(64),
                vout: 0,
                script_sig: String::new(),
                pub_key: String::new(),
                sequence: 0,
            }],
            vec![TxOutput {
                value: 10,
                script_pub_key: wallet.get_address(),
            }],
        );

        tx.sign(&wallet);
        assert!(tx.verify());
    }
}
