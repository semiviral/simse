use axum::Router;

pub fn build() -> Router {
    Router::new().route("/v2/server", axum::routing::get(|| async {
        
    }))
}