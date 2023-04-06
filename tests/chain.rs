#[cfg(test)]
mod chain_tests {

    use std::collections::VecDeque;

    use rustychain::Block;
    use rustychain::Chain;

    #[test]
    fn test_have_errors() {
        // create some random chain...
        let mut block0 = Block::new(0, String::from("First"));
        block0.update_hash();
        let mut block1 = Block::new(1, String::from("Second"));
        block1.prev = block0.hash;
        block1.update_hash();
        let mut block2 = Block::new(2, String::from("Third"));
        block2.prev = block1.hash;
        block2.update_hash();
        let mut chain = Chain {
            blocks: vec![block0, block1, block2],
            status: true,
            queue: VecDeque::new(),
        };

        assert_eq!(chain.have_errors(), None);
        let block2 = chain.blocks.last_mut().unwrap();
        block2.nonce = 1;
        assert_eq!(chain.have_errors(), Some(2));
        chain.status = false;
        assert_eq!(chain.have_errors(), None);
        chain.status = true;

        let block1 = chain.blocks.get_mut(1).unwrap();
        block1.id = 0;
        assert_eq!(chain.have_errors(), Some(1));
        let block1 = chain.blocks.get_mut(1).unwrap();
        block1.id = 1;
        block1.prev = block1.prev.map(|x| x + 1);
        assert_eq!(chain.have_errors(), Some(1));
    }

    #[test]
    fn test_try_add() {
        let mut block0 = Block::new(0, String::from("First"));
        block0.update_hash();
        let mut block1 = Block::new(1, String::from("Second"));
        block1.prev = block0.hash;
        block1.update_hash();
        let mut block2 = Block::new(2, String::from("Third"));
        block2.prev = block1.hash;

        let mut chain = Chain {
            blocks: vec![block0],
            status: false,
            queue: VecDeque::from(vec![block1, block2]),
        };
        assert!(!chain.try_add());
        assert_eq!(chain.blocks.len(), 1);
        assert_eq!(chain.queue.len(), 2);

        chain.status = true;
        assert!(chain.try_add());
        assert_eq!(chain.blocks.len(), 2);
        assert_eq!(chain.queue.len(), 1);
        assert_eq!(chain.have_errors(), None);

        assert!(chain.try_add());
        assert_eq!(chain.blocks.len(), 3);
        assert_eq!(chain.queue.len(), 0);

        assert!(!chain.try_add()); // queue is empty
        assert_eq!(chain.blocks.len(), 3);
        assert_eq!(chain.queue.len(), 0);

        assert_eq!(chain.have_errors(), Some(2)); // block 2 don't updated hash
        chain.blocks.last_mut().unwrap().update_hash();
        assert_eq!(chain.have_errors(), None);
    }
}