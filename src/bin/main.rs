use std::{
    collections::hash_map::DefaultHasher,
    error::Error,
    hash::{Hash, Hasher},
    time::Duration,
};

use async_std::io;

use clap::{arg, Parser};
use futures::{prelude::*, StreamExt};
use libp2p::{
    gossipsub, identity, mdns,
    swarm::{NetworkBehaviour, SwarmBuilder, SwarmEvent},
    PeerId,
};
use log::{error, info};

use rustychain::{Block, Chain, Message, Node};
use tokio::{
    sync::{
        broadcast,
        mpsc::{self},
    },
    task::{self},
};

#[derive(NetworkBehaviour)]
struct MyBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::async_io::Behaviour,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("00"), value_parser = validate_hex)]
    difficulty: String,
}

fn validate_hex(s: &str) -> Result<String, String> {
    let lowwered = s.to_lowercase();
    let valid_chars = "0123456789abcdef";
    if lowwered.chars().all(|c| valid_chars.contains(c)) {
        Ok(lowwered.to_string())
    } else {
        Err(String::from("Input contains invalid characters"))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Argument with difficult of blocks
    let args = Args::parse();

    // Enable logging
    pretty_env_logger::init();

    // PeedId creating
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    info!("Local peer: \"{local_peer_id}\"");

    // DNS, web-sockets, security...
    let transport = libp2p::development_transport(local_key.clone()).await?;

    // Custom network
    #[derive(NetworkBehaviour)]
    struct MyBehaviour {
        gossipsub: gossipsub::Behaviour,
        mdns: mdns::async_io::Behaviour,
    }

    // Validation for msg
    let message_id_fn = |message: &gossipsub::Message| {
        let mut s = DefaultHasher::new();
        message.data.hash(&mut s);
        gossipsub::MessageId::from(s.finish().to_string())
    };

    // Config fabric
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .message_id_fn(message_id_fn) // validation
        .duplicate_cache_time(Duration::from_secs(1)) // cache time
        .heartbeat_interval(Duration::from_secs(600)) // Smaller spam in logger
        .validation_mode(gossipsub::ValidationMode::Strict) // Message signing
        .build()
        .expect("Valid config");

    // Network behaviour
    let mut gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(local_key),
        gossipsub_config,
    )
    .expect("Correct configuration");

    // Topic
    let topic = gossipsub::IdentTopic::new("hash-net");
    gossipsub.subscribe(&topic)?;

    // Create a Swarm to manage peers and events
    let mut swarm = {
        let mdns = mdns::async_io::Behaviour::new(mdns::Config::default(), local_peer_id)?;
        let behaviour = MyBehaviour { gossipsub, mdns };
        SwarmBuilder::with_async_std_executor(transport, behaviour, local_peer_id).build()
    };

    // Reading from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();

    // Listen ports
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    info!("Now you can enter transactions via STDIN");
    info!("Transactions will be sended to other nodes");

    // IPC for node
    let (tx_net, rx_node) = mpsc::channel::<Message>(64);
    let (tx_node, mut rx_net) = mpsc::channel::<Message>(64);
    let (_tx_cancel, rx_cancel) = broadcast::channel(1);

    // Run task with blockchain node
    let mut node = Node::new(Chain::new(), tx_node, rx_node, rx_cancel, args.difficulty);
    let _task = task::spawn(async move {
        node.run().await;
    });

    // ls command flag
    let mut ls_flag = false;

    // Return
    loop {
        tokio::select! {
            line = stdin.select_next_some() => {

                let mut line = line.expect("Stdin not to close");
                if line == "ls"  {
                    ls_flag = true;
                    if let Err(e) = tx_net.send(Message::ChainRequest).await {
                        error!("Can't send data to host node: {e}");
                    }
                }
                if line.starts_with("=") && line.len() > 1 {
                    line.remove(0);


                    let block = Block::new(0, line);
                    let msg = Message::NewBlock(block);
                    let serded = serde_json::to_string(&msg).expect("Message is serializible");

                    info!("[Host] {}",msg);
                    if let Err(e) = tx_net.send(msg).await {
                        error!("Can't send data to host node: {e}");
                    }

                    if let Err(e) = swarm
                        .behaviour_mut().gossipsub
                        .publish(topic.clone(), serded) {
                        error!("Publish error around sending new transaction to other hosts: {e:?}");
                    }
                }
            },
            msg = rx_net.recv().fuse() => {
                match msg {
                    Some(msg) => {

                        if let Message::ChainResponce(chain) = msg.clone() {
                            if ls_flag {
                                info!("Chain:\r\n + {}",chain);
                                ls_flag = false;
                                continue;
                            }
                        }
                        info!("[Host] {}",msg);

                        let serded = serde_json::to_string(&msg).expect("Message is serializible");

                        if let Err(e) = swarm
                        .behaviour_mut().gossipsub
                        .publish(topic.clone(), serded) {
                        error!("Publish error around sending some data to other hosts: {e:?}");
                 }
                    },
                    None => {}
                }
            },
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, multiaddr) in list {
                        info!("mDNS discovered a new peer: {multiaddr} {peer_id} ");
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _multiaddr) in list {
                        info!("mDNS discover peer has expired: {peer_id}");
                        swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: peer_id,
                    message_id: _id,
                    message,
                })) =>  {
                    // recieve message from remote
                    let msg = String::from_utf8_lossy(&message.data);
                    let msg : Message = serde_json::from_str(&msg).expect("Message should be desializeble");
                    let peer = peer_id.to_string();
                    info!("[Remote {peer}]: {msg}");

                    // put it in host
                    if let Err(e)  = tx_net.send(msg).await {
                        error!("Can't send data to host node: {e}");
                    }

                },
                _ => {}
            }
        }
    }
}
