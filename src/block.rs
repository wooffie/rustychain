use core::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Block {
    pub id: u64,
    pub data: String,
    pub hash: [u8; 32],
    pub prev: [u8; 32],
    pub nonce: u64,
}

impl Block {
    pub fn new(id: u64, data: String) -> Self {
        Self {
            id,
            data,
            hash: [0u8; 32],
            prev: [0u8; 32],
            nonce: 0,
        }
    }

    pub fn calc_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.id.to_be_bytes());
        hasher.update(self.data.clone());
        hasher.update(self.prev);
        hasher.update(self.nonce.to_be_bytes());
        return hasher
            .finalize()
            .as_slice()
            .try_into()
            .expect("SHA256 output must be 256 bit");
    }

    pub fn update_hash(&mut self) {
        self.hash = self.calc_hash();
    }

    pub fn validate_hash(&self) -> bool {
        self.hash == self.calc_hash()
    }

    pub fn string_hash(&self) -> String {
        hex::encode(self.hash)
    }

    pub fn string_prev(&self) -> String {
        hex::encode(self.prev)
    }

    pub fn equals(&self, other: &Self) -> bool {
        self.id == other.id
            && self.data == other.data
            && self.hash == other.hash
            && self.prev == other.prev
            && self.nonce == other.nonce
    }

    pub fn preequals(&self, other: &Self) -> bool {
        self.id == other.id && self.data == other.data && self.prev == other.prev
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "#{} hash: {}, previous: {}, data \"{}\", nonce {}",
            self.id,
            self.string_hash(),
            self.string_prev(),
            self.data,
            self.nonce,
        )
    }
}
