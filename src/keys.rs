use std::collections::HashMap;

use anyhow::Result;
use base64::prelude::*;
use ed25519_dalek::Keypair;
use rand::{rngs::OsRng, Rng};
use tokio::sync::OnceCell;

static SIGNING_KEYS: OnceCell<HashMap<String, Keypair>> = OnceCell::const_new();
static OLD_SIGNING_KEYS: OnceCell<HashMap<String, Keypair>> = OnceCell::const_new();

pub async fn load_keys(config: &crate::config::Config) -> Result<()> {
    assert!(
        SIGNING_KEYS.get().is_none(),
        "signing key has already been loaded"
    );

    let mut signing_keys = HashMap::new();

    for keyfile_path in &config.server.keys.keyfiles {
        debug!(
            "Loading signing key from file: {}",
            keyfile_path.to_string_lossy()
        );

        let file_data = tokio::fs::read_to_string(keyfile_path).await?;
        let (key_id, key_material_base64) = file_data.split_once(' ').ok_or(anyhow::Error::msg(
            r#"keyfile is improperly formatted; expected "keyname keymaterial" format"#,
        ))?;

        let key_material = BASE64_STANDARD_NO_PAD.decode(key_material_base64)?;
        let keypair = Keypair::from_bytes(key_material.as_ref())?;

        debug!(
            "Signing key: {}: {}",
            &key_id,
            base64::engine::general_purpose::STANDARD_NO_PAD.encode(keypair.public)
        );

        signing_keys.insert(key_id.to_owned(), keypair);
    }

    signing_keys.shrink_to_fit();
    SIGNING_KEYS.set(signing_keys).unwrap ();

    Ok(())
}

pub async fn generate_keypair(args: crate::args::GenKeyArguments) -> Result<()> {
    println!("Generating Ed25519 keypair...");
    let keypair = Keypair::generate(&mut OsRng).to_bytes();
    let bytes_base64 = BASE64_STANDARD_NO_PAD.encode(keypair);

    let keypair_id = args.name.unwrap_or_else(|| {
        let mut keypair_id_bytes = [0u8; 8];
        OsRng.fill(&mut keypair_id_bytes);

        BASE64_STANDARD_NO_PAD.encode(keypair_id_bytes)
    });

    println!("Writing keypair to file...");
    tokio::fs::write(&args.path, format!("{keypair_id} {bytes_base64}")).await?;
    println!("Keypair written to file: {}", args.path.to_string_lossy());

    Ok(())
}
