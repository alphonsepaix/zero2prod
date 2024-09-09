use crate::routes::*;
use axum::{
    body::Body,
    http::Request,
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::Level;

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/", get(greet))
        .route("/health_check", get(health_check))
        .route("/reply", post(reply))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<Body>| {
                let request_id = uuid::Uuid::new_v4();
                tracing::span!(
                    Level::DEBUG,
                    "request",
                    method = tracing::field::display(request.method()),
                    uri = tracing::field::display(request.uri()),
                    version = tracing::field::debug(request.version()),
                    request_id = tracing::field::display(request_id)
                )
            }),
        )
}

pub async fn run(listener: TcpListener, pool: PgPool) -> Result<(), std::io::Error> {
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, router().with_state(pool)).await
}
