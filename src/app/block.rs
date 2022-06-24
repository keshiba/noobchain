use serde::{
    Serialize, Deserialize,
};
use chrono::Utc;

use crate::app::utils;
use crate::app::DIFFICULTY_PREFIX;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub id: u64,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: i64,
    pub data: String,
    pub nonce: u64
}

impl Block {
    pub fn new(id: u64, previous_hash: String, data: String) -> Self {
        let now = Utc::now();
        let (nonce, hash) = mine_hash(id, now.timestamp(), &previous_hash, &data);

        return Self { 
            id: id, 
            hash: hash, 
            previous_hash: previous_hash, 
            timestamp: now.timestamp(), 
            data: data, 
            nonce: nonce 
        };
    }
}

fn mine_hash(id: u64, timestamp: i64, previous_hash: &str, data: &str) -> (u64, String) {
    info!("mining block...");
    let mut nonce = 0;

    loop {
        if nonce % 100000 == 0 {
            info!("nonce: {}", nonce);
        }

        let hash = utils::calculate_hash(id, timestamp, previous_hash, data, nonce);
        let hash_binary = utils::hash_to_binary_representation(&hash);

        if hash_binary.starts_with(DIFFICULTY_PREFIX) {
            info!(
                "mined! nonce: {}, hash: {}, binary hash: {}",
                nonce,
                hex::encode(&hash),
                hash_binary
            );

            return (nonce, hex::encode(hash));
        }

        nonce += 1;
    }
}