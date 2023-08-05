mod args;
mod config;
mod notify;
mod routes;

use clap::Parser;
use once_cell::sync::{Lazy, OnceCell};

static ARGS: Lazy<args::Arguments> = Lazy::new(args::Arguments::parse);

static AGENT_STRING: Lazy<String> =
    Lazy::new(|| format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")));

fn get_config() -> &'static config::Config {
    static CONFIG: OnceCell<config::Config> = OnceCell::new();

    CONFIG.get_or_init(|| {
        let config_path = &ARGS.config_path;

        assert!(
            config_path.exists(),
            "Specified configuration file does not exist: {:?}",
            config_path
        );

        let config_str =
            std::fs::read_to_string(config_path).expect("configuration does not exist at path");

        serde_yaml::from_str(&config_str).expect("failed to parse configuration")
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    if notify::spawn_notifier().is_err() {
        tracing::error!(
            "Failed to spawn notifier task; this is likely because it has already been spawned."
        );
    }

    // let mut watcher = notify::recommended_watcher(|res| match res {
    //     Ok(event) => trace!("FS event fired: {:?}", event),
    //     Err(err) => todo!(),
    // })
    // .unwrap();

    // let watch_path = std::path::Path::new("blast");
    // watcher
    //     .watch(watch_path, notify::RecursiveMode::Recursive)
    //     .unwrap();
    // info!("Watching path for changes: {:?}", watch_path);
    let server_config = &get_config().server;

    let socket = std::net::SocketAddr::from((server_config.address, server_config.port));
    tracing::info!("Server listening @{}", socket);

    axum::Server::bind(&socket)
        .serve(routes::build().into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("shutdown notified");

    tracing::warn!("Shutdown signalled.");
}
