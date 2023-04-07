use log::{error, info, warn};

use tokio::{
    sync::{
        broadcast,
        mpsc::{self, Receiver, Sender},
    },
    task::{self},
};

use crate::{Block, Chain, Message};

/// Represents a node in the blockchain network.
#[derive(Debug)]
pub struct Node {
    /// The blockchain maintained by this node.
    chain: Chain,
    /// The sender used to send messages to other nodes in the network.
    tx: Sender<Message>,
    /// The receiver used to receive messages from other nodes in the network.
    rx: Receiver<Message>,
    /// The receiver used to make graceful shutdown of thread.
    rx_cancel: broadcast::Receiver<()>,
    /// The difficulty level for mining blocks in the blockchain.
    difficult: String,
}

impl Node {
    /// Constructs a new Node instance with the given chain, tx, rx, and difficult.
    ///
    /// # Arguments
    ///
    /// * chain - A `Chain` instance to be used by the Node.
    /// * tx - A `Sender<Message>` instance to send messages to other nodes.
    /// * rx - A `Receiver<Message>` instance to receive messages from other nodes.
    /// * rx_cancel - A 'broadcast::Receiver<()>' intance to receive shutdown message.
    /// * difficult - A String representing the difficulty of the mining process for the Node.
    ///
    /// # Returns
    ///
    /// A new Node instance with the given parameters.
    pub fn new(
        chain: Chain,
        tx: Sender<Message>,
        rx: Receiver<Message>,
        rx_cancel: broadcast::Receiver<()>,
        difficult: String,
    ) -> Self {
        Self {
            chain,
            tx,
            rx,
            rx_cancel,
            difficult,
        }
    }

    /// Asynchronously runs the node, listening for incoming messages on the receive channel `self.rx`.
    /// Messages received are processed based on their type, which can be one of the following:
    ///
    /// - `Message::NewBlock(block)`: Adds the new block to the node's chain queue.
    ///
    /// - `Message::ChainRequest`: Sends a chain response containing the node's current chain to the requesting node.
    ///
    /// - `Message::ChainResponse(chain)`: Compares the received chain with the current chain, replacing the current chain if the received chain is longer and contains no errors.
    ///
    /// - `Message::MinedBlock(block)`: Compares the received block with the node's current last block, replacing the last block with the received block if it has a higher block ID and passes validation. If the received block has the same block ID as the last block and the node is not currently mining, then the node takes the received block as its own.
    ///
    /// If the node is currently mining and the mining process is complete, the newly mined block is added to the node's chain queue and a new mining process is started.
    ///
    /// The `run` function never returns, but instead diverges into an infinite loop that continues to process incoming messages.
    pub async fn run(&mut self) {
        let (tx_node, rx) = mpsc::channel::<(Block, String)>(16);
        let (tx, mut rx_node) = mpsc::channel::<([u8; 32], u64)>(16);

        let (cancel_tx, cancel_rx) = broadcast::channel(1);

        let _task = task::spawn(async move {
            nonce_worker(rx, tx, cancel_rx).await;
        });

        loop {
            tokio::select! {
            _ = self.rx_cancel.recv() => {
                // graceful shutdown
                cancel_tx.send(()).unwrap();
                return;
            },
            msg = self.rx.recv() => {
                if let Some(message) = msg {
                    match message {
                        Message::NewBlock(block) => {
                            self.chain.add_queue(block);
                        },
                        Message::ChainRequest => {
                            if let Err(e) = self.tx.send(Message::ChainResponce(self.chain.clone())).await {
                                error!("Sending chain error: {:?}",e);
                            } else {
                                info!("Serve chain request");
                            }

                        },
                        Message::ChainResponce(chain) => {
                            if chain.have_errors().is_none(){
                                if chain.blocks.len() > self.chain.blocks.len() {
                                    self.chain = chain;
                                    warn!("Taking chain from another node!");
                                }
                            } else {
                                error!("Chain from another node has errors!")
                            }
                        }
                        Message::MinedBlock(block) => {

                            if block.hash != block.calc_hash() && block.string_hash().ends_with(&self.difficult){
                                warn!("Reciever block with wrong hash field: {}",block);
                                continue;
                            }

                            let last = self.chain.blocks.last_mut();
                            match last {
                                Some(last) => {

                                    if block.id == last.id && self.chain.status && block.preequals(last) && block.hash < last.hash {
                                            last.hash = block.hash;
                                            last.nonce = block.nonce;
                                            info!("Replaced host block with remote block");
                                    }

                                    if block.id == last.id && !self.chain.status && block.preequals(last) {
                                        last.hash = block.hash;
                                        last.nonce = block.nonce;
                                        self.chain.status = true;
                                        info!("Took remote block");
                                    }

                                    if block.id > last.id {
                                        if let Err(e) = self.tx.send(Message::ChainRequest).await {
                                            error!("Sending chain request error: {:?}",e);
                                        }
                                    }

                                    if let Some(id) = self.chain.have_errors() {
                                        warn!("Host chain have erros! Requesting remote");
                                            for _ in id..self.chain.blocks.len(){
                                                let block = self.chain.blocks.pop().unwrap();
                                                self.chain.queue.push_front(block);
                                            }
                                            if let Err(e) = self.tx.send(Message::ChainRequest).await{
                                                error!("Chain request sending error: {:?}",e);
                                            }
                                    }
                                },
                                None => {
                                    info!("Host chain in empty, requesting remote");
                                    if let Err(e) = self.tx.send(Message::ChainRequest).await{
                                        error!("Chain request sending error: {:?}",e);
                                    }
                                }
                            }
                        },
                    }
                } else {
                    error!("Error around net and node connection");
                }
            },
            nonce = rx_node.recv() => {
                if let Some(nonce) = nonce {
                    if !self.chain.status {
                        let mut cloned_block = self.chain.blocks.last().unwrap().clone();
                        cloned_block.hash = nonce.0;
                        cloned_block.nonce = nonce.1;
                        if cloned_block.hash == cloned_block.calc_hash(){
                            let mut last = self.chain.blocks.last_mut().unwrap();
                            last.hash = cloned_block.hash;
                            last.nonce = cloned_block.nonce;
                            self.chain.status = true;

                            if let Err(e) = self.tx.send(Message::MinedBlock(last.clone())).await{
                                error!("Sending error: {:?}",e)
                            }
                            info!("Mined!");

                        }
                    }
                }
            }
            }

            if self.chain.status {
                self.chain.status = !self.chain.try_add();

                if !self.chain.status {
                    let last_block = self.chain.blocks.last().unwrap(); // we know!
                    let diff = self.difficult.clone();
                    if let Err(e) = tx_node.send((last_block.clone(), diff)).await {
                        warn!("Can't send data to worker: {e}");
                    }
                }
            }
        }
    }
}

/// Mines the nonce for the given block and difficulty string using a Tokio task.
///
/// The function takes a receiving end of a channel, `rx`, which is used to receive a tuple of
/// the block and the difficulty string. It also takes a sending end of a channel, `tx`, which is
/// used to send back the resulting hash and nonce. Lastly, it takes a receiving end of a broadcast
/// channel, `cancel_rx`, which is used to gracefully shutdown the function.
pub async fn nonce_worker(
    mut rx: Receiver<(Block, String)>,
    tx: Sender<([u8; 32], u64)>,
    mut cancel_rx: broadcast::Receiver<()>,
) {
    let mut flag = false;
    let mut block = Block::new(0, String::from("Dummy"));
    let mut diff = String::from("zzzz");
    loop {
        tokio::select! {
            _ = cancel_rx.recv() => {
                // graceful shutdown
                return;
            },
            m = rx.recv() => {
                if let Some((b,s)) = m {
                    block = b;
                        diff = s;
                        flag = true;
                }

            },
            _ = tokio::time::sleep(tokio::time::Duration::from_nanos(1)) => {
                if flag{
                    let nonce = rand::random::<u64>();
                    block.nonce = nonce;
                    block.update_hash();
                    if block.string_hash().ends_with(&diff){
                        if let Err(e) = tx.send((block.hash, nonce)).await {
                            error!("Error around worker {:?}",e);
                        }
                        flag = false;
                    }
                }
            },
        }
    }
}
