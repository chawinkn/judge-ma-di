pub mod healthcheck;
pub mod task;

use axum::{
    extract::{DefaultBodyLimit, MatchedPath, Request},
    http::Method,
    routing::get,
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
        .route(
            "/api/tasks/:id",
            // TODO: Auth
            get(task::get_manifest)
                .post(task::upload_task)
                .delete(task::delete_task)
                .layer(DefaultBodyLimit::max(10 * 1024 * 1024)),
        )
        .route("/api/tasks/:id/manifest", get(task::get_manifest))
        .route("/api/tasks/:id/desc", get(task::get_desc))
        .route("/api/tasks/:id/testcases", get(task::get_task_testcases))
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
