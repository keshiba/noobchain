use sha2::{Sha256, Digest};

pub fn hash_to_binary_representation(hash: &[u8]) -> String {

    let mut res: String = String::default();

    for c in hash {
        res.push_str(&format!("{:b}", c));
    }

    return res;
}

pub fn calculate_hash(id: u64, timestamp: i64, previous_hash: &str, data: &str, nonce: u64) -> Vec<u8> {

    let data = serde_json::json!({
        "id": id,
        "previous_hash": previous_hash,
        "data": data,
        "timestamp": timestamp,
        "nonce": nonce
    });

    let mut hasher = Sha256::new();
    hasher.update(data.to_string().as_bytes());

    return hasher.finalize().as_slice().to_owned();
}