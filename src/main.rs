mod args;
mod config;
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
        Arguments::Crypto(CryptoArguments::Genkey(args)) => genkey(args).await,

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

    let signing_key = load_signing_key(config.server.keys.keyfile).await?;

    debug!("Signing key: {:?}", signing_key);

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

async fn load_signing_key(
    keyfile_path: impl AsRef<std::path::Path>,
) -> Result<(String, ed25519_dalek::Keypair)> {
    let keyfile_path = keyfile_path.as_ref();

    debug!(
        "Loading signing key from file: {}",
        keyfile_path.to_string_lossy()
    );

    let file_data = tokio::fs::read_to_string(keyfile_path).await?;
    let (key_name, key_material_base64) = file_data.split_once(' ').ok_or(anyhow::Error::msg(
        r#"keyfile is improperly formatted; expected "keyname keymaterial" format"#,
    ))?;

    let key_material =
        base64::engine::general_purpose::STANDARD_NO_PAD.decode(key_material_base64)?;
    let keypair = ed25519_dalek::Keypair::from_bytes(key_material.as_ref())?;

    Ok((key_name.to_owned(), keypair))
}

async fn genkey(args: GenKeyArguments) -> Result<()> {
    use base64::engine::general_purpose::STANDARD_NO_PAD;
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
