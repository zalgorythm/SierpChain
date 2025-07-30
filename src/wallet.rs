use ed25519_dalek::{Signer, Signature, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Sha256, Digest};

const VERSION: u8 = 0x00;
const CHECKSUM_LEN: usize = 4;

/// A wallet that holds a signing key.
#[derive(Debug)]
pub struct Wallet {
    pub signing_key: SigningKey,
}

impl Wallet {
    /// Creates a new `Wallet` with a randomly generated signing key.
    pub fn new() -> Self {
        let mut csprng = OsRng{};
        let signing_key: SigningKey = SigningKey::generate(&mut csprng);
        Wallet { signing_key }
    }

    /// Returns the wallet's public key (verifying key).
    pub fn get_public_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Signs a message with the wallet's private key.
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }

    /// Returns the wallet's address.
    ///
    /// The address is a Base58Check encoded version of the public key hash.
    /// 1. Prepend version byte to public key hash
    /// 2. Double SHA256 hash the result
    /// 3. Take first 4 bytes as checksum
    /// 4. Append checksum to the version-prefixed hash
    /// 5. Base58 encode the result
    pub fn get_address(&self) -> String {
        let pub_key_hash = self.hash_pub_key();
        let mut versioned_payload = vec![VERSION];
        versioned_payload.extend_from_slice(&pub_key_hash);

        let checksum = Self::checksum(&versioned_payload);
        let mut full_payload = versioned_payload;
        full_payload.extend_from_slice(&checksum);

        bs58::encode(full_payload).into_string()
    }

    /// Hashes the public key using SHA-256.
    fn hash_pub_key(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(self.get_public_key().as_bytes());
        hasher.finalize().to_vec()
    }

    /// Creates a checksum for a payload.
    fn checksum(payload: &[u8]) -> Vec<u8> {
        let mut first_hasher = Sha256::new();
        first_hasher.update(payload);
        let first_hash = first_hasher.finalize();

        let mut second_hasher = Sha256::new();
        second_hasher.update(&first_hash);
        let second_hash = second_hasher.finalize();

        second_hash[0..CHECKSUM_LEN].to_vec()
    }
}

impl Default for Wallet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Verifier;

    #[test]
    fn test_wallet_creation() {
        let wallet = Wallet::new();
        assert!(wallet.get_address().starts_with("1"));
    }

    #[test]
    fn test_wallet_signing() {
        let wallet = Wallet::new();
        let message = b"hello, world";
        let signature = wallet.sign(message);
        let public_key = wallet.get_public_key();
        assert!(public_key.verify(message, &signature).is_ok());
    }
}
