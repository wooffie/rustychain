#[cfg(test)]
mod node_tests {
    use rustychain::{nonce_worker, Block, Chain, Message, Node};
    use tokio::sync::{broadcast, mpsc};

    #[tokio::test]
    async fn test_node() {
        let (tx_test, rx_node) = mpsc::channel::<Message>(10);
        let (tx_node, mut rx_test) = mpsc::channel::<Message>(10);
        let (tx_cancel, rx_cancel) = broadcast::channel(1);

        // small difficult for tests...
        let diff = String::from("0");
        let mut node = Node::new(Chain::new(), tx_node, rx_node, rx_cancel, diff.clone());

        let handle = tokio::task::spawn(async move {
            node.run().await;
        });

        let data = String::from("Some data");
        let block = Block::new(1337, data);

        tx_test
            .send(Message::NewBlock(block.clone()))
            .await
            .unwrap();
        let msg = rx_test.recv().await.unwrap();

        let prev_hash;

        if let Message::MinedBlock(res) = msg {
            assert_eq!(res.data, block.data);
            assert_eq!(res.id, 0);
            assert!(res.string_hash().ends_with(&diff));
            prev_hash = res.hash;
        } else {
            panic!("Expected MinedBlock, but got: {:?}", msg);
        }

        tx_test
            .send(Message::NewBlock(block.clone()))
            .await
            .unwrap();
        let msg = rx_test.recv().await.unwrap();

        if let Message::MinedBlock(res) = msg {
            assert_eq!(res.data, block.data);
            assert_eq!(res.id, 1);
            assert!(res.string_hash().ends_with(&diff));
            assert_eq!(res.prev, prev_hash);
        } else {
            panic!("Expected MinedBlock, but got: {:?}", msg);
        }

        // shutdown
        tx_cancel.send(()).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_worker() {
        let (tx, rx) = mpsc::channel(10);
        let (result_tx, mut result_rx) = mpsc::channel(10);
        let (cancel_tx, cancel_rx) = broadcast::channel(1);

        let handle = tokio::task::spawn(nonce_worker(rx, result_tx, cancel_rx));

        let mut block = Block::new(1, String::from("test"));
        let diff = String::from("0");
        tx.send((block.clone(), diff.clone())).await.unwrap();

        let (hash, nonce) = result_rx.recv().await.unwrap();
        block.hash = hash;
        block.nonce = nonce;
        assert!(block.validate_hash());
        assert!(block.string_hash().ends_with(&diff));

        cancel_tx.send(()).unwrap();
        handle.await.unwrap();
    }
}
