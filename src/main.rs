mod args;
mod config;
mod keys;
mod notify;
mod routes;

use anyhow::Result;
use args::*;
use base64::Engine;
use clap::Parser;
use once_cell::sync::Lazy;

#[macro_use]
extern crate tracing;

static AGENT_STRING: Lazy<String> =
    Lazy::new(|| format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")));

#[tokio::main]
async fn main() {
    match Arguments::parse() {
        Arguments::Crypto(CryptoArguments::Genkey(args)) => keys::generate_keypair(args).await,

        Arguments::Server(args) => {
            setup_logging(&args).expect("failed to configure global logger");

            run_server(args).await
        }
    }
    .expect("error occurred");

    trace!("Reached safe shutdown point.");
}

async fn run_server(args: ServerArguments) -> Result<()> {
    let config: config::Config = {
        assert!(
            &args.config_path.exists(),
            "Specified configuration file does not exist: {:?}",
            &args.config_path
        );

        let config_str = std::fs::read_to_string(&args.config_path)?;
        serde_yaml::from_str(&config_str)?
    };

    debug!("Loading signing key...");
    keys::load_keys(&config).await?;

    if let Err(err) = notify::spawn_notifier(&config.notifiers).await {
        error!("Failed to spawn notifier: {err:?}");
    }

    let socket = std::net::SocketAddr::from((config.server.address, config.server.port));
    info!("Server listening @ {}", socket);

    axum::Server::bind(&socket)
        .serve(routes::build().into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn setup_logging(args: &ServerArguments) -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(args.log_level)
        .with_thread_ids(args.log_thread_id)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;
    info!("Logging configured and ready.");

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("shutdown notified");

    info!("Shutdown signalled.");
}
