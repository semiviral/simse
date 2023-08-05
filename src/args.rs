use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(rename_all = "snake_case")]
pub enum Arguments {
    Server(ServerArguments),

    #[command(subcommand)]
    Crypto(CryptoArguments),
}

#[derive(Parser)]
#[command(rename_all = "snake_case")]
pub struct ServerArguments {
    #[arg(long = "config.file", env = "SIMSE_CONFIG_PATH", required = true)]
    pub config_path: PathBuf,

    #[arg(long = "log.level", env = "SIMSE_LOG_LEVEL", default_value = "info")]
    pub log_level: tracing::Level,

    #[arg(long = "log.thread_id", env = "SIMSE_LOG_THREAD_ID")]
    pub log_thread_id: bool,
}

#[derive(clap::Subcommand)]
#[command(rename_all = "snake_case")]
pub enum CryptoArguments {
    Genkey(GenKeyArguments),
}

#[derive(Parser)]
pub struct GenKeyArguments {
    #[arg(long, short)]
    pub name: Option<String>,

    pub path: PathBuf,
}
