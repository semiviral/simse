use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    storage: Storage,
    notifier: Notifier,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Password {
    Value(String),
    Path(PathBuf),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Storage {
    kind: StorageKind,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageKind {
    PostgreSql {
        host: String,
        port: u16,
        database: String,
        schema: String,
        username: String,
        password: Password,
    },

    Local {
        path: String,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Notifier {
    startup_check: bool,
    file: Option<String>,
    smtp: Option<SmtpNotifier>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SmtpNotifier {
    require_tls: bool,
    host: String,
    port: u16,
    timeout: usize,
    username: String,
    password: Password,
    sender: String,
    subject: String,
}
