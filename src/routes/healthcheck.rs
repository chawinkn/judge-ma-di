use crate::error::{json_response, ResponseCode};
use axum::response::IntoResponse;
use deadpool_postgres::Pool;

pub async fn health_check(pool: Pool) -> impl IntoResponse {
    let db_ok = match pool.get().await {
        Ok(client) => client.query_one("SELECT 1", &[]).await.is_ok(),
        Err(_) => false,
    };

    if db_ok {
        json_response(ResponseCode::Ok)
    } else {
        json_response(ResponseCode::ServiceUnavailable)
    }
}
