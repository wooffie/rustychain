use core::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The `Block` struct represents a block in the blockchain.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Block {
    /// The ID of the block.
    pub id: u64,
    /// The data stored in the block.
    pub data: String,
    /// The hash of the block.
    pub hash: [u8; 32],
    /// The hash of the previous block in the chain.
    pub prev: [u8; 32],
    /// The nonce used to mine the block.
    pub nonce: u64,
}

impl Block {
    /// Creates a new `Block` with the given `id` and `data`.
    ///
    /// # Example
    ///
    /// ```
    /// use rustychain::Block;
    ///
    /// let block = Block::new(1, "Hello, world!".to_owned());
    ///
    /// assert_eq!(block.id, 1);
    /// assert_eq!(block.data, "Hello, world!");
    /// ```
    pub fn new(id: u64, data: String) -> Self {
        Self {
            id,
            data,
            hash: [0u8; 32],
            prev: [0u8; 32],
            nonce: 0,
        }
    }

    /// Calculates the SHA256 hash for the block and returns it
    /// 
    /// # Examples
    /// 
    /// ```
    /// use rustychain::Block;
    /// 
    /// let block = Block::new(0, "Hello World!".to_owned());
    /// let hash = block.calc_hash();
    /// ```
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

    /// Recalculate the hash of the current block based on its current data, previous block hash,
    /// and nonce. The updated hash is then stored in the `hash` field of the block.
    ///
    /// # Examples
    ///
    /// ```
    /// use rustychain::Block;
    ///
    /// let mut block = Block::new(0, String::from("Genesis Block"));
    /// let hash = block.calc_hash();
    /// block.update_hash();
    /// assert_eq!(block.hash.len(), 32);
    /// assert_eq!(block.hash,hash);
    /// ```
    pub fn update_hash(&mut self) {
        self.hash = self.calc_hash();
    }

    /// Validates the current block's hash against its calculated hash.
    ///
    /// # Examples
    ///
    /// ```
    /// use rustychain::Block;
    ///
    /// let mut block = Block::new(1, "Hello World".to_string());
    /// block.update_hash();
    /// assert_eq!(block.validate_hash(), true);
    /// ```   
    pub fn validate_hash(&self) -> bool {
        self.hash == self.calc_hash()
    }

    /// Returns a hexadecimal string representation of the block's hash.
    ///
    /// # Example
    ///
    /// ```
    /// use rustychain::Block;
    ///
    /// let mut block = Block::new(1, "Hello world!".to_string());
    /// block.update_hash();
    /// let hash_str = block.string_hash();
    ///
    /// assert_eq!(hash_str.len(), 64);
    /// ```
    pub fn string_hash(&self) -> String {
        hex::encode(self.hash)
    }

    /// Returns a hexadecimal string representation of the previous block's hash.
    ///
    /// # Example
    ///
    /// ```
    /// use rustychain::Block;
    ///
    /// let mut block = Block::new(1, "Hello world!".to_string());
    /// block.update_hash();
    /// let hash_str = block.string_prev();
    ///
    /// assert_eq!(hash_str.len(), 64);
    /// ```
    pub fn string_prev(&self) -> String {
        hex::encode(self.prev)
    }

    /// Compares the current block with another block for equality.
    ///
    /// # Arguments
    ///
    /// * `other` - A reference to another `Block` instance to compare against.
    ///
    /// # Returns
    ///
    /// Returns `true` if the two blocks have equal values for `id`, `data`,
    /// `hash`, `prev`, and `nonce`. Otherwise, returns `false`.
    /// # Examples
    ///
    /// ```
    /// use rustychain::Block;
    ///
    /// let block1 = Block::new(1, String::from("Data"));
    /// let block2 = Block::new(1, String::from("Data"));
    /// let block3 = Block::new(2, String::from("Data"));
    ///
    /// assert!(block1.equals(&block2));
    /// assert!(!block1.equals(&block3));
    /// ```
    pub fn equals(&self, other: &Self) -> bool {
        self.id == other.id
            && self.data == other.data
            && self.hash == other.hash
            && self.prev == other.prev
            && self.nonce == other.nonce
    }

    /// Checks if the `id`, `data` and `prev` fields of two `Block` instances are equal.
    ///
    /// # Arguments
    ///
    /// * `other` - A reference to the `Block` instance to compare with.
    ///
    /// # Examples
    ///
    /// ```
    /// use rustychain::Block;
    ///
    /// let block1 = Block::new(1, String::from("Block 1"));
    /// let block2 = Block::new(2, String::from("Block 2"));
    ///
    /// assert!(block1.preequals(&block1));
    /// assert!(!block1.preequals(&block2));
    /// ```
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
