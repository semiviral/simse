use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, Router},
};

pub fn build() -> Router {
    Router::new().route("/v1/version", get(version).options(version))
}

async fn version() -> impl IntoResponse {
    (StatusCode::OK, crate::agent_string())
}
