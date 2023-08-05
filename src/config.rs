use lettre::message::Mailbox;
use serde::Deserialize;
use std::{
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
    str::FromStr,
};
use versions::SemVer;

fn serde_true() -> bool {
    true
}

fn default_server_address() -> IpAddr {
    IpAddr::V4(Ipv4Addr::LOCALHOST)
}

fn default_server_port() -> u16 {
    9005
}

fn default_smtp_timeout() -> u64 {
    3000
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server: Server,
    pub storage: StorageConfig,
    pub notifiers: NotifierConfig,
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
    pub old_keys: Option<Vec<ServerKeysOld>>,
}

#[derive(Debug, Deserialize)]
pub struct Server {
    pub name: String,

    #[serde(deserialize_with = "deserialize_address")]
    #[serde(default = "default_server_address")]
    pub address: IpAddr,
    #[serde(default = "default_server_port")]
    pub port: u16,

    pub keys: ServerKeys,
}

#[derive(Debug, Deserialize)]
pub struct StorageConfig {
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
pub struct NotifierConfig {
    #[serde(default = "serde_true")]
    pub startup_check: bool,

    #[serde(default)]
    pub smtp: Option<SmtpNotifierConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SmtpTlsMode {
    StartTls,
    ForceTls,
    Off,
}

#[derive(Debug, Deserialize)]
pub struct SmtpNotifierConfig {
    pub host: String,
    pub port: u16,
    pub tls: SmtpTlsMode,
    pub sender: Mailbox,
    pub subject: String,
    pub username: String,
    pub passfile: PathBuf,

    #[serde(default = "default_smtp_timeout")]
    pub timeout: u64,
}

struct AddressVisitor;
impl<'de> serde::de::Visitor<'de> for AddressVisitor {
    type Value = IpAddr;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an IP v4 or v6 address (or 'localhost')")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        match v {
            "localhost" => Ok(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            v => IpAddr::from_str(v).map_err(|_| {
                serde::de::Error::unknown_field(v, &["an IP v4 or v6 address (or 'localhost')"])
            }),
        }
    }
}

fn deserialize_address<'de, D: serde::Deserializer<'de>>(obj: D) -> Result<IpAddr, D::Error> {
    obj.deserialize_str(AddressVisitor)
}
