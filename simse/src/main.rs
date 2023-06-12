mod notifiers;

use anyhow::Result;
use once_cell::sync::OnceCell;
use simse_config::Config;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::trace;
use yew::prelude::*;

static CONFIG: OnceCell<Config> = OnceCell::new();

fn get_config() -> &'static Config {
    CONFIG.get().unwrap()
}

trait AsyncReadWrite: AsyncRead + AsyncWrite {}
impl<RW: AsyncRead + AsyncWrite> AsyncReadWrite for RW {}

trait AsyncBufReadWriteUnpin: AsyncRead + AsyncWrite + Unpin {}
impl<RWU: AsyncRead + AsyncWrite + Unpin> AsyncBufReadWriteUnpin for RWU {}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    CONFIG
        .set({
            let config = read_config().await.unwrap();
            trace!("Config: {:#?}", config);

            config
        })
        .unwrap();

    if notifiers::spawn_notifier().is_err() {
        tracing::error!(
            "Failed to spawn notifier task; this is likely because it has already been spawned."
        );
    }

    yew::Renderer::<App>::new().render();

    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
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

    // let app = Router::new().route("/", get(root));

    // let addr = SocketAddr::from(([127, 0, 0, 1], 3006));

    // info!("listening on {}", addr);
    // axum::Server::bind(&addr)
    //     .serve(app.into_make_service())
    //     .await
    //     .unwrap();
}

#[function_component(App)]
fn app() -> Html {
    html! {
        <>
            <h1>{ "RustConf Explorer" }</h1>
            <div>
                <h3>{"Videos to watch"}</h3>
                <p>{ "John Doe: Building and breaking things" }</p>
                <p>{ "Jane Smith: The development process" }</p>
                <p>{ "Matt Miller: The Web 7.0" }</p>
                <p>{ "Tom Jerry: Mouseless development" }</p>
            </div>
            <div>
                <h3>{ "John Doe: Building and breaking things" }</h3>
                <img src="https://via.placeholder.com/640x360.png?text=Video+Player+Placeholder" alt="video thumbnail" />
            </div>
        </>
    }
}

async fn read_config() -> Result<Config> {
    let config_path = option_env!("SIMSE_CONFIG_PATH").unwrap_or("config.toml");
    let config_file = tokio::fs::read_to_string(config_path).await?;
    let config = toml::from_str(&config_file)?;

    Ok(config)
}
