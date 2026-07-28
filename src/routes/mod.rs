pub mod desc;
pub mod healthcheck;
pub mod task;

use axum::{
    extract::{DefaultBodyLimit, MatchedPath, Request},
    http::Method,
    routing::{delete, get, post},
    Router,
};
use deadpool_postgres::Pool;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info_span;

pub fn build(pool: Pool) -> Router {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_origin(Any);

    Router::new()
        .route(
            "/api/healthcheck",
            get(move || healthcheck::health_check(pool.clone())),
        )
        .route("/api/task/:id", get(task::get_task_testcases))
        .route(
            "/api/task/:id",
            post(task::upload_task).layer(DefaultBodyLimit::max(1024 * 1000 * 10)),
        )
        .route("/api/task/:id", delete(task::delete_task))
        .route("/api/desc/:id", get(desc::get_desc))
        .route("/api/task/manifest/:id", get(task::get_manifest))
        .layer(cors)
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                // Log the matched route's path (with placeholders not filled in).
                // Use request.uri() or OriginalUri if you want the real path.
                let matched_path = request
                    .extensions()
                    .get::<MatchedPath>()
                    .map(MatchedPath::as_str);

                info_span!(
                    "http_request",
                    method = ?request.method(),
                    matched_path,
                    some_other_field = tracing::field::Empty,
                )
            }),
        )
}
