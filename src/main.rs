mod config;

use anyhow::{Context, Error, Result};
use axum::{
    routing::{get, post},
    Router,
};
use config::Config;
use notify::Watcher;
use std::{net::SocketAddr, path::PathBuf};
use tracing::{info, trace};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = read_config().await.unwrap();
    info!("Config: {:#?}", config);

    let mut watcher = notify::recommended_watcher(|res| match res {
        Ok(event) => trace!("FS event fired: {:?}", event),
        Err(err) => todo!(),
    })
    .unwrap();

    let watch_path = std::path::Path::new("blast");
    watcher
        .watch(watch_path, notify::RecursiveMode::Recursive)
        .unwrap();
    info!("Watching path for changes: {:?}", watch_path);

    let app = Router::new().route("/", get(root));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3006));
    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn read_config() -> Result<Config> {
    let config_path = option_env!("SIMSE_CONFIG_PATH").unwrap_or("config.toml");
    let config_file = tokio::fs::read_to_string(config_path)
        .await
        .with_context(|| format!("failed to read config from path: {}", config_path))?;

    toml::from_str(&config_file).with_context(|| "failed to parse valid config TOML from file")
}
