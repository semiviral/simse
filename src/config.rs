use lettre::message::Mailbox;
use serde::Deserialize;
use std::{net::Ipv4Addr, path::PathBuf};
use versions::SemVer;

fn serde_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server: Server,
    pub storage: Storage,
    pub notifiers: Notifiers,
}

#[derive(Debug, Deserialize)]
pub struct Tls {
    pub server_name: String,
    pub skip_verify: bool,
    pub min_version: SemVer,
    pub max_version: SemVer,
}

#[derive(Debug, Deserialize)]
pub struct ServerKeysOld {
    pub value: String,
    pub expired_ts: u64,
}

#[derive(Debug, Deserialize)]
pub struct ServerKeys {
    pub keyfile: PathBuf,
    pub oldkeys: Vec<ServerKeysOld>,
}

#[derive(Debug, Deserialize)]
pub struct Server {
    pub name: String,

    pub address: Ipv4Addr,
    pub port: u16,

    pub keys: ServerKeys,
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
        passfile: PathBuf,
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
pub struct Notifiers {
    #[serde(default = "serde_true")]
    pub startup_check: bool,

    #[serde(default)]
    pub smtp: Option<SmtpNotifier>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SmtpTlsMode {
    StartTls,
    ForceTls,
    Off,
}

#[derive(Debug, Deserialize)]
pub struct SmtpNotifier {
    pub host: String,
    pub port: u16,
    pub tls: SmtpTlsMode,
    pub timeout: u64,
    pub username: String,
    pub passfile: PathBuf,
    pub to: Mailbox,
    pub from: Mailbox,
    pub subject: String,
    // tls: Option<Tls>,
}
