use serde::Deserialize;
use std::{net::Ipv4Addr, path::PathBuf};
use versions::SemVer;

use crate::notifiers::smtp::Sender;

fn serde_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub storage: Storage,
    pub notifier: Notifier,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Password {
    Value(String),
    Path(PathBuf),
}

#[derive(Debug, Deserialize)]
pub struct Tls {
    pub server_name: String,
    pub skip_verify: bool,
    pub min_version: SemVer,
    pub max_version: SemVer,
}

#[derive(Debug, Deserialize)]
pub struct Server {
    pub address: Ipv4Addr,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct Storage {
    #[serde(flatten, default)]
    pub kind: StorageKind,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageKind {
    PostgreSql {
        host: String,
        port: u16,
        database: String,
        schema: String,
        username: String,
        password_value: Option<String>,
        password_file: Option<PathBuf>,
        // tls: Option<Tls>,
    },

    Local {
        path: String,
    },
}

impl Default for StorageKind {
    fn default() -> Self {
        Self::Local {
            path: "storage.db".to_owned(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Notifier {
    #[serde(default = "serde_true")]
    pub startup_check: bool,

    #[serde(default)]
    pub file: Option<String>,

    #[serde(default)]
    pub smtp: Option<SmtpNotifier>,
}

#[derive(Debug, Deserialize)]
pub struct SmtpNotifier {
    pub host: String,
    pub port: u16,
    pub timeout: u64,
    pub username: String,
    pub password_value: Option<String>,
    pub password_file: Option<PathBuf>,
    pub sender: Sender,
    pub subject: String,
    // tls: Option<Tls>,
}
