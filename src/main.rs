use judge_ma_di::{db, init_tracing, routes, worker};
use std::process::exit;
use tracing::{info, warn};

#[tokio::main]
async fn main() {
    init_tracing();
    let env_name = std::env::var("APP_ENV")
        .or_else(|_| std::env::var("ENV"))
        .unwrap_or_else(|_| "development".to_string());
    info!(env = %env_name, "Starting judge-ma-di...");

    let postgres_url = std::env::var("POSTGRES_URL").expect("POSTGRES_URL not found");
    let pool = db::create_pool(&postgres_url).unwrap_or_else(|err| {
        warn!("Failed to create PostgreSQL pool: {:?}", err);
        exit(1);
    });

    let worker_pool = pool.clone();
    let worker_handler = tokio::spawn(async move {
        if let Err(err) = worker::run_worker(worker_pool).await {
            warn!("Worker failed: {:?}", err);
            exit(1);
        }
    });

    let app = routes::build(pool);

    let port = "0.0.0.0:5000";
    let listener = tokio::net::TcpListener::bind(port).await.unwrap();

    let api_handler = tokio::spawn(async move {
        info!(" Server is starting on: {:?}", port);
        axum::serve(listener, app).await.unwrap();
    });

    tokio::select! {
        _ = worker_handler => {
            warn!("Worker handler kaboom!!!");
            exit(1);
        }
        _ = api_handler => {
            warn!("Api handler kaboom!!!");
            exit(1);
        }
    }
}
