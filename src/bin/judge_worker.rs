use judge_ma_di::{db, worker};
use std::process::exit;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[tokio::main]
async fn main() {
    init_tracing();
    info!("Starting judge-worker...");

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
