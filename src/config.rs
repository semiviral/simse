use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use versions::SemVer;

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
pub struct Tls {
    server_name: String,
    skip_verify: bool,
    min_version: SemVer,
    max_version: SemVer,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Storage {
    #[serde(flatten)]
    kind: StorageKind,
}

#[allow(clippy::large_enum_variant)]
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
        // tls: Option<Tls>,
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
    host: String,
    port: u16,
    timeout: usize,
    username: String,
    password: Password,
    sender: String,
    subject: String,
    // tls: Option<Tls>,
}
