use core::fmt;

use serde::{Serialize, Deserialize};

use crate::{Block, Chain};

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    MinedBlock(Block),
    NewBlock(Block),
    ChainRequest,
    ChainResponce(Chain),
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::MinedBlock(block) => write!(f, "MinedBlock({})", block),
            Message::NewBlock(block) => write!(f, "New Transaction({})", block.data),
            Message::ChainRequest => write!(f, "ChainRequest"),
            Message::ChainResponce(chain) => write!(f, "Chain Response:\r\n {}", chain),
        }
    }
}