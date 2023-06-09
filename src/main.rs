use anyhow::Error;
use axum::{
    routing::{get, post},
    Router,
};
use notify::Watcher;
use std::net::SocketAddr;
use tracing::{info, trace};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

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
