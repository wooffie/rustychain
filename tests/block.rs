#[cfg(test)]
mod block_tests {

    use rustychain::Block;

    #[test]
    fn test_new() {
        let block = Block::new(0, String::from("Data"));
        assert_eq!(block.id, 0);
        assert_eq!(block.data, String::from("Data"));
        assert_eq!(block.hash, [0u8; 32]);
        assert_eq!(block.prev, [0u8; 32]);
        assert_eq!(block.nonce, 0);
    }

    #[test]
    fn test_calc_hash() {
        let mut block1 = Block {
            id: 100,
            data: String::from("This is the first block"),
            hash: [1u8; 32],
            prev: [0u8; 32],
            nonce: 1,
        };
        let block2 = block1.clone();
        assert_eq!(block1.calc_hash(), block2.calc_hash());
        let prev = block1.calc_hash();
        block1.nonce = 0;
        assert_ne!(block1.calc_hash(), prev);
        assert_ne!(block1.calc_hash(), block2.calc_hash());
    }

    #[test]
    fn test_update_hash() {
        let mut block = Block::new(1337, String::from("Leet block!"));
        assert_ne!(block.calc_hash(), block.hash);
        block.update_hash();
        assert_eq!(block.calc_hash(), block.hash);
    }

    #[test]
    fn test_validate_hash(){
        let mut block = Block::new(1337, String::from("Leet block!"));
        assert!(!block.validate_hash());
        block.update_hash();
        assert!(block.validate_hash());
    }

    #[test]
    fn test_equals(){
        let mut block1= Block::new(1337, String::from("Leet block!"));
        block1.update_hash();

        let mut block2 = block1.clone();
        assert!(block1.equals(&block2));

        block2 = block1.clone();
        block2.data = String::from("Not leet block!");
        assert!(!block1.equals(&block2));

        block2 = block1.clone();
        block2.hash = block1.hash.map(|v| v + 1);
        assert!(!block1.equals(&block2));

        block2 = block1.clone();
        block2.prev = block1.prev.map(|v| v + 1);
        assert!(!block1.equals(&block2));

        block2 = block1.clone();
        block2.nonce = block1.nonce + 1;
        assert!(!block1.equals(&block2));
    }

    #[test]
    fn test_preequals(){
        let mut block1= Block::new(1337, String::from("Leet block!"));
        block1.update_hash();

        let mut block2 = block1.clone();
        assert!(block1.preequals(&block2));

        block2 = block1.clone();
        block2.data = String::from("Not leet block!");
        assert!(!block1.preequals(&block2));

        block2 = block1.clone();
        block2.hash = block1.hash.map(|v| v + 1);
        assert!(block1.preequals(&block2));

        block2 = block1.clone();
        block2.prev = block1.prev.map(|v| v + 1);
        assert!(!block1.preequals(&block2));

        block2 = block1.clone();
        block2.nonce = block1.nonce + 1;
        assert!(block1.preequals(&block2));
    }
}
