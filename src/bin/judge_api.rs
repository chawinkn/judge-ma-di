use judge_ma_di::{db, routes};
use std::process::exit;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=debug,tower_http=debug,axum::rejection=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[tokio::main]
async fn main() {
    init_tracing();
    info!("Starting judge-api...");

    let postgres_url = std::env::var("POSTGRES_URL").expect("POSTGRES_URL not found");
    let pool = db::create_pool(&postgres_url).unwrap_or_else(|err| {
        warn!("Failed to create PostgreSQL pool: {:?}", err);
        exit(1);
    });

    let app = routes::build(pool);

    let port = "0.0.0.0:5000";
    let listener = tokio::net::TcpListener::bind(port).await.unwrap();
    info!("Server listening on: {:?}", port);
    axum::serve(listener, app).await.unwrap();
}
