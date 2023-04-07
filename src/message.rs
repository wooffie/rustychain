use core::fmt;

use serde::{Deserialize, Serialize};

use crate::{Block, Chain};

/// A message sent between nodes in the blockchain network.
///
/// Message can be sent NET<->NET or NODE<->NET
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// A newly mined block that is ready to be added to the blockchain.
    MinedBlock(Block),
    /// A new block received from another node that needs to be calculated and added to the blockchain.
    NewBlock(Block),
    /// A request for the entire blockchain.
    ChainRequest,
    /// A response to a `ChainRequest`, containing the current state of the blockchain.
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
