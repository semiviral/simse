mod args;
mod config;
mod notify;
mod routes;

use args::*;
use clap::Parser;
use once_cell::sync::Lazy;

#[macro_use]
extern crate tracing;

static AGENT_STRING: Lazy<String> =
    Lazy::new(|| format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")));

#[tokio::main]
async fn main() {
    match Arguments::parse() {
        Arguments::Crypto(CryptoArguments::Genkey(args)) => genkey(args).await,

        Arguments::Server(args) => {
            let subscriber = tracing_subscriber::fmt()
                .with_max_level(args.log_level)
                .with_thread_ids(args.log_thread_id)
                .compact()
                .finish();

            tracing::subscriber::set_global_default(subscriber)
                .expect("failed to configure global logger");
            info!("Logging configured and ready.");

            run_server(args).await
        }
    }
    .expect("error occurred");

    trace!("Reached safe shutdown point.");
}

async fn run_server(args: ServerArguments) -> anyhow::Result<()> {
    let config: config::Config = {
        assert!(
            &args.config_path.exists(),
            "Specified configuration file does not exist: {:?}",
            &args.config_path
        );

        let config_str = std::fs::read_to_string(&args.config_path)?;
        serde_yaml::from_str(&config_str)?
    };

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

async fn genkey(args: GenKeyArguments) -> anyhow::Result<()> {
    use base64::engine::{general_purpose::STANDARD_NO_PAD, Engine};
    use ed25519_dalek::Keypair;
    use rand::{rngs::OsRng, Rng};

    println!("Generating Ed25519 keypair...");
    let keypair = Keypair::generate(&mut OsRng).to_bytes();
    let bytes_base64 = STANDARD_NO_PAD.encode(keypair);

    let keypair_id = args.name.unwrap_or_else(|| {
        let mut keypair_id_bytes = [0u8; 8];
        OsRng.fill(&mut keypair_id_bytes);

        STANDARD_NO_PAD.encode(keypair_id_bytes)
    });

    println!("Writing keypair to file...");
    tokio::fs::write(&args.path, format!("{keypair_id} {bytes_base64}")).await?;
    println!("Keypair written to file: {}", args.path.to_string_lossy());

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("shutdown notified");

    tracing::warn!("Shutdown signalled.");
}
