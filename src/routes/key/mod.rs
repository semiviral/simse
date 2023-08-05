use std::collections::HashMap;

pub mod routes;
mod signer;

pub struct VerifyKey {
    key: String
}

pub struct OldKey {
    expired_ts: u64,
    key: String
}

pub struct ServerKeyResponse {
    server_name: String,
    keys: HashMap<String, VerifyKey>,
    valid_until_ts: u64,
    ols_verify_keys: HashMap<String, OldKey>,
}

