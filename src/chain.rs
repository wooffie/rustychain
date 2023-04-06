use core::fmt;
use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::Block;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chain {
    pub blocks: Vec<Block>,
    pub status: bool,
    pub queue: VecDeque<Block>,
}

impl Chain {
    pub fn new() -> Self {
        Chain {
            blocks: vec![],
            status: true,
            queue: VecDeque::new(),
        }
    }

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

    pub fn add_queue(&mut self, mut block: Block) {
        block.id = (self.blocks.len() + self.queue.len()) as u64;
        self.queue.push_back(block);
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
