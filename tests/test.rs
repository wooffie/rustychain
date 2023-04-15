#[cfg(test)]
mod test {
    use futures::StreamExt;
    use libp2p::swarm::{NetworkBehaviour, SwarmBuilder, SwarmEvent};
    use libp2p::{gossipsub, identity, mdns, PeerId};
    use rustychain::{Block, Chain, Message};
    use std::collections::hash_map::DefaultHasher;
    use std::collections::HashMap;
    use std::error::Error;
    use std::hash::{Hash, Hasher};
    use std::io::Write;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn network_test() -> Result<(), Box<dyn Error>> {
        // Compile program to run it in test
        println!("Compile program");
        let mut build = Command::new("cargo")
            .arg("build")
            .spawn()
            .expect("Can't start compilation on program");

        let status = build.wait().expect("failed to wait for child process");
        assert!(status.success(), "Can't compile program!!!");

        // Creating mock
        println!("Mock starting...");
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        let transport = libp2p::development_transport(local_key.clone()).await?;

        #[derive(NetworkBehaviour)]
        struct MyBehaviour {
            gossipsub: gossipsub::Behaviour,
            mdns: mdns::async_io::Behaviour,
        }

        let message_id_fn = |message: &gossipsub::Message| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            gossipsub::MessageId::from(s.finish().to_string())
        };

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .message_id_fn(message_id_fn) // validation
            .duplicate_cache_time(Duration::from_secs(1)) // cache time
            .heartbeat_interval(Duration::from_secs(600)) // Smaller spam in logger
            .validation_mode(gossipsub::ValidationMode::Strict) // Message signing
            .build()
            .expect("Valid config");

        let mut gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key),
            gossipsub_config,
        )
        .expect("Correct configuration");

        let topic = gossipsub::IdentTopic::new("hash-net");
        gossipsub.subscribe(&topic)?;

        let mut swarm = {
            let mdns = mdns::async_io::Behaviour::new(mdns::Config::default(), local_peer_id)?;
            let behaviour = MyBehaviour { gossipsub, mdns };
            SwarmBuilder::with_async_std_executor(transport, behaviour, local_peer_id).build()
        };

        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
        println!("Mock started");
        // wait for network up
        sleep(Duration::from_secs(5)).await;
        println!("Starting 3 nodes!");

        let mut children = Vec::new();
        for _ in 0..3 {
            let child = Command::new("cargo")
                .arg("run")
                .arg("--")
                .arg("-d")
                .arg("00")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("failed to execute child process");
            children.push(child);
        }

        // to get up all childs
        sleep(Duration::from_secs(5)).await;

        println!("3 nodes started!");

        // test data
        let all_data = vec!["Genesis", "First", "Second", "Third", "Fourth"];
        let mut data = all_data.clone();
        data.reverse();
        let mut all_blocks: Vec<Block> = vec![];
        let mut chains: HashMap<String, Chain> = HashMap::new();
        let mut collect_flag = false;

        loop {
            tokio::select! {
                event = swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                        for (peer_id, multiaddr) in list {
                            println!("mDNS discovered a new peer: {multiaddr} {peer_id} ");
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        }
                    },
                    SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                        for (peer_id, _multiaddr) in list {
                            println!("mDNS discover peer has expired: {peer_id}");
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
                        if let Message::ChainResponce(chain) = msg.clone() {
                            chains.insert(peer, chain);
                        }
                        if let Message::MinedBlock(block) = msg.clone(){
                            all_blocks.push(block);
                        }

                    },
                    _ => {},
                },
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(7)) => {
                    println!("New activity");
                    let b = data.pop();
                    if let Some(s) = b {
                        println!("Send new data");
                        let msg = Message::NewBlock(Block::new(0, s.to_string()));
                        let serded = serde_json::to_string(&msg).expect("Message is serializible");

                        if let Err(e) = swarm
                            .behaviour_mut()
                            .gossipsub
                            .publish(topic.clone(), serded)
                        {
                        println!("Publish error around sending new transaction to other hosts: {e:?}");
                         }
                    } else {
                        if collect_flag {
                            break;
                        } else {
                            println!("Send chain requests");
                            let msg = Message::ChainRequest;
                            let serded = serde_json::to_string(&msg).expect("Message is serializible");
                            if let Err(e) = swarm
                            .behaviour_mut()
                            .gossipsub
                            .publish(topic.clone(), serded)
                        {
                        println!("Publish error around sending new transaction to other hosts: {e:?}");
                         }
                        }
                        collect_flag = true;
                    }
                },
            }
        }

        for mut child in children {
            println!("Shutdown child!");
            let stdin = child.stdin.take().unwrap();

            let writer_thread = thread::spawn(move || {
                let mut stdin_writer = stdin;
                writeln!(stdin_writer, "exit").unwrap();
            });

            let status = child.wait().expect("failed to wait for child process");
            assert!(status.success());
            writer_thread.join().unwrap();
        }

        // Testing blocks
        for block in all_blocks {
            assert_eq!(block.hash, block.calc_hash());
            assert!(block.string_hash().ends_with("00"));
        }

        // Test all incomming chains
        for (_k,v) in chains{
            assert_eq!(v.have_errors(),None);
        }

        Ok(())
    }
}
