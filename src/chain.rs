use core::fmt;
use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::Block;

/// A blockchain that consists of a vector of `Block`s and maintains a queue of `Block`s yet to be
/// appended to the chain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chain {
    /// The vector of blocks that make up the blockchain.
    pub blocks: Vec<Block>,
    /// The current status of the blockchain. True - can get new blocks. False - calculating hash of last block yet.
    pub status: bool,
    /// The queue of blocks that are yet to be appended to the blockchain.
    pub queue: VecDeque<Block>,
}

impl Chain {
    /// Constructs a new, empty blockchain `Chain`.
    ///
    /// This function creates a new, empty `Chain` instance with no blocks or transactions.
    ///
    /// # Examples
    ///
    /// ```
    /// use rustychain::Chain;
    ///
    /// let chain = Chain::new();
    /// ```
    pub fn new() -> Self {
        Chain {
            blocks: vec![],
            status: true,
            queue: VecDeque::new(),
        }
    }

    /// Checks if the chain contains any errors.
    ///
    /// Returns `None` if the chain is valid, or the index of the first invalid block
    /// encountered in the chain.
    ///
    /// # Examples
    ///
    /// ```
    /// use rustychain::Chain;
    /// use rustychain::Block;
    ///
    /// let mut chain = Chain::new();
    ///
    /// assert_eq!(chain.have_errors(), None);
    ///
    /// let block1 = Block::new(0,"Some data".to_owned());
    /// let block2 = Block::new(1,"Some data".to_owned());
    /// chain.status = false;
    /// chain.blocks.push(block1);
    /// assert_eq!(chain.have_errors(), None);
    /// chain.blocks.push(block2);
    /// assert_eq!(chain.have_errors(), Some(0));
    ///
    /// ```
    pub fn have_errors(&self) -> Option<usize> {
        let mut len = self.blocks.len();
        if len == 0 {
            return None;
        }
        if !self.status {
            len = len - 1;
        }
        for i in 0..len {
            let block = self.blocks.get(i).expect("Must have block with this id");
            if block.id != i as u64 || !block.validate_hash() {
                return Some(i);
            }
        }
        for i in 0..len {
            if i == 0 {
                continue;
            }
            let block = self.blocks.get(i).expect("Must have block with this id");
            let prev = self
                .blocks
                .get(i - 1)
                .expect("Must have block with this id");

            if block.prev != prev.hash {
                return Some(i);
            }
        }
        None
    }

    /// Attempts to add a new block to the chain. If the chain is currently in an invalid state,
    /// this function will return false.
    ///
    /// Returns true if the new block is added to the chain, false otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use rustychain::Chain;
    /// use rustychain::Block;
    ///
    /// let mut chain = Chain::new();
    /// let block = Block::new(1, "data1".to_string());
    ///
    /// chain.queue.push_back(block);
    ///
    /// assert_eq!(chain.try_add(), true);
    /// assert_eq!(chain.try_add(), false);
    /// assert_eq!(chain.blocks.len(), 1);
    ///
    /// let block2 = Block::new(2, "data2".to_string());
    ///
    /// chain.queue.push_back(block2);
    ///
    /// assert_eq!(chain.try_add(), true);
    /// assert_eq!(chain.blocks.len(), 2);
    /// ```
    pub fn try_add(&mut self) -> bool {
        if self.status {
            // ready
            if self.queue.is_empty() {
                // check queue
                return false;
            } else {
                let mut block = self.queue.pop_front().unwrap();
                let prev = match self.blocks.last() {
                    Some(a) => a.hash,
                    None => [0u8; 32],
                };
                block.prev = prev;
                block.id = self.blocks.len() as u64;
                self.blocks.push(block);
                return true;
            }
        } else {
            false
        }
    }

    /// Adds a new block to the end of the queue.
    ///
    /// The ID of the block will be set to the sum of the number of blocks in the chain
    /// and the number of blocks in the queue. The block will be added to the back of the
    /// queue, waiting to be added to the chain by calling the `try_add()` method.
    ///
    /// # Arguments
    ///
    /// * `block` - A `Block` instance representing the block to be added to the queue.
    ///
    /// # Example
    ///
    /// ```
    /// use rustychain::Chain;
    /// use rustychain::Block;
    ///
    /// let mut chain = Chain::new();
    /// let block = Block::new(1, "First Block".to_owned());
    /// chain.add_queue(block);
    /// assert_eq!(chain.queue.len(), 1);
    /// ```
    pub fn add_queue(&mut self, mut block: Block) {
        block.id = (self.blocks.len() + self.queue.len()) as u64;
        self.queue.push_back(block);
    }
}

impl Default for Chain {
    fn default() -> Self {
        Chain::new()
    }
}

impl fmt::Display for Chain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match write!(f, "Status: {}\r\n", self.status) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        for block in self.blocks.iter() {
            match write!(f, "{}\r\n", block) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }
        for block in self.queue.iter() {
            match write!(f, "#-{} \"{}\'\r\n", block.id, block.data) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }
}
