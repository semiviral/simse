pub mod federation;

use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, Router},
};

pub fn build() -> Router {
    Router::new()
        .fallback(fallback)
        .route("/health", get(health).options(health))
        .nest("/_matrix/federation/", federation::routes::build())
}

async fn fallback(uri: axum::http::Uri) -> impl IntoResponse {
    (StatusCode::NOT_FOUND, format!("No route for URI: {}", uri))
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "I'm alive!")
}
