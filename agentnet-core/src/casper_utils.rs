pub fn public_key_to_account_hash(pk_hex: &str) -> String {
    let clean = pk_hex.trim();
    if clean.starts_with("account-hash-") || clean.starts_with("hash-") {
        return clean.to_string();
    }
    if clean.len() == 64 {
        return format!("account-hash-{}", clean);
    }
    if (clean.starts_with("01") && clean.len() == 66)
        || (clean.starts_with("02") && clean.len() == 68)
    {
        if let Ok(bytes) = hex::decode(clean) {
            use blake2::digest::Digest;
            use blake2::digest::consts::U32;
            type Blake2b256 = blake2::Blake2b<U32>;

            let tag = if bytes[0] == 1 {
                b"ed25519\0".as_slice()
            } else {
                b"secp256k1\0".as_slice()
            };
            let mut hasher = Blake2b256::new();
            hasher.update(tag);
            hasher.update(&bytes[1..]);
            let res = hasher.finalize();
            let account_hash_hex = hex::encode(res);
            return format!("account-hash-{}", account_hash_hex);
        }
    }
    format!("account-hash-{}", clean)
}

pub fn recommended_price_motes(_domain: &str, total: u32, processing_time_ms: u64) -> u64 {
    let base_price = 5_000_000_000u64;

    let speed_multiplier = if processing_time_ms < 5000 {
        1.2
    } else if processing_time_ms < 15000 {
        1.0
    } else if processing_time_ms < 30000 {
        0.8
    } else {
        0.6
    };

    (base_price as f64 * (total as f64 / 100.0) * speed_multiplier) as u64
}
