pub mod block;
mod utils;

use block::Block;
use chrono::Utc;

const DIFFICULTY_PREFIX: &str = "00";

pub struct App { 
    pub blocks: Vec<Block>
}

impl App {

    /// Initializes a new instance of the app with an empty blockchain
    /// # Examples
    /// ```rust
    /// let mut app = App::new();
    /// ```
    pub fn new() -> Self {
        Self { blocks: vec![] }
    }

    /// Creates and adds a genesis block to the blockchain. This is a
    /// hardcoded block added to the chain to signify it's starting point.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let mut app = App::new();
    /// 
    /// app.genesis();
    /// assert!(app.blocks.len() == 1);
    /// ```
    /// 
    pub fn genesis(&mut self) {

        let genesis_block = Block {
            id: 0,
            timestamp: Utc::now().timestamp(),
            previous_hash: String::from("genesis"),
            data: String::from("genesis block"),
            nonce: 1955,
            hash: "836f1092881c0a62ba187d4ad760553171b2ddd3b705c55c1baea613cadc760e".to_string() // November 5
        };

        self.blocks.push(genesis_block);
    }

    /// Adds a new block into the blockchain after verifying that the
    /// block is valid
    /// 
    pub fn try_add_block(&mut self, new_block: Block) {
        
        let last_block = self.blocks.last().expect("blocks is empty");
        
        if self.is_block_valid(&new_block, last_block) {
            self.blocks.push(new_block);
        } else {
            error!("couldn't add new block since it's invalid");
        }
    }

    /// Checks if the block is valid in the chain
    /// 
    fn is_block_valid(&self, block: &Block, last_block: &Block) -> bool {
    
        if block.previous_hash != last_block.hash {

            warn!("block with id {} has wrong previous hash", block.id);
            return false;
        } 

        let hash_hex = hex::decode(&block.hash).expect("unable to decode hash to hex");
        let hash_binary: String = utils::hash_to_binary_representation(&hash_hex);
        
        if hash_binary.starts_with(DIFFICULTY_PREFIX) == false {

            warn!("block with id {} has invalid difficulty", block.id);
            return false;
        } 
        
        if block.id != last_block.id + 1 {

            warn!("block with id {} and last block {} are not contiguous", block.id, last_block.id);
            return false;
        } 
        
        let block_hash = utils::calculate_hash(
            block.id, 
            block.timestamp, 
            &block.previous_hash, 
            &block.data, 
            block.nonce
        );

        if hex::encode(block_hash) != block.hash {

            warn!("block with id {} has invalid hash", block.id);
            return false;
        }

        return true;
    }

    fn is_chain_valid(&self, chain: &[Block]) -> bool {

        for i in 1..chain.len() {
            let first_block = chain.get(i - 1).expect("has to exist");
            let second_block = chain.get(i).expect("has to exist");

            if !self.is_block_valid(second_block, first_block) {
                return false;
            }
        }

        return true;
    }

    fn choose_chain(&mut self, local: Vec<Block>, remote: Vec<Block>) -> Vec<Block> {

        let is_local_valid = self.is_chain_valid(&local);
        let is_remote_valid = self.is_chain_valid(&remote);

        if is_local_valid && is_remote_valid {
            if local.len() >= remote.len() {
                return local;
            } else {
                return remote;
            }
        } else if is_local_valid {
            return local;
        } else if is_remote_valid {
            return remote;
        } else {
            panic!("Both local and remote chains are invalid");
        }
    }

    pub fn print_chain(&self) {
        
        for block in &self.blocks {
            info!("{:?}", utils::hash_to_binary_representation(&hex::decode(&block.hash).expect("bleh")));
        }
    }
}