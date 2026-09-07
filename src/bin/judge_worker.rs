use judge_ma_di::{db, init_tracing, worker};
use std::process::exit;
use tracing::{info, warn};

#[tokio::main]
async fn main() {
    init_tracing();
    let env_name = std::env::var("APP_ENV")
        .or_else(|_| std::env::var("ENV"))
        .unwrap_or_else(|_| "development".to_string());
    info!(env = %env_name, "Starting judge-worker...");

    let postgres_url = std::env::var("POSTGRES_URL").expect("POSTGRES_URL not found");
    let pool = db::create_pool(&postgres_url).unwrap_or_else(|err| {
        warn!("Failed to create PostgreSQL pool: {:?}", err);
        exit(1);
    });

    if let Err(err) = worker::run_worker(pool).await {
        warn!("Worker failed: {:?}", err);
        exit(1);
    }
}
