use std::collections::HashMap;

pub mod routes;

struct VerifyKey {
    key: String
}

struct OldKey {
    expired_ts: u64,
    key: String
}

struct ServerKeyResponse {
    server_name: String,
    keys: HashMap<String, VerifyKey>,
    valid_until_ts: u64,
    ols_verify_keys: HashMap<String, OldKey>,
}