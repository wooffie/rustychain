use log::{error, info, warn};

use tokio::{
    sync::{
        broadcast,
        mpsc::{self, Receiver, Sender},
    },
    task::{self},
};

use crate::{Block, Chain, Message};

#[derive(Debug)]
pub struct Node {
    chain: Chain,
    tx: Sender<Message>,
    rx: Receiver<Message>,
    rx_cancel: broadcast::Receiver<()>,
    difficult: String,
}

impl Node {
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

                                    if block.id == last.id && self.chain.status && block.preequals(last) {
                                        if block.hash < last.hash {
                                            last.hash = block.hash;
                                            last.nonce = block.nonce;
                                            info!("Replaced host block with remote block");
                                        }
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

                                    match self.chain.have_errors() {
                                        Some(id) => {
                                            warn!("Host chain have erros! Requesting remote");
                                            for _ in id..self.chain.blocks.len(){
                                                let block = self.chain.blocks.pop().unwrap();
                                                self.chain.queue.push_front(block);
                                            }
                                            if let Err(e) = self.tx.send(Message::ChainRequest).await{
                                                error!("Chain request sending error: {:?}",e);
                                            }
                                        },
                                        None => {},

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
                match nonce{
                    Some(nonce) => {

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
                    },
                    None => {
                    },
                }
            }
            }

            if self.chain.status == true {
                self.chain.status = !self.chain.try_add();

                if self.chain.status == false {
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
