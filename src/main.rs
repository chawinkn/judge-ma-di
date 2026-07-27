use judge_ma_di::{db, queue, routes};
use std::process::exit;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
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
    info!(" Starting...");

    let postgres_url = std::env::var("POSTGRES_URL").expect("POSTGRES_URL not found");
    let pool = db::create_pool(&postgres_url).unwrap_or_else(|err| {
        warn!("Failed to create PostgreSQL pool: {:?}", err);
        exit(1);
    });

    let rbmq_url = std::env::var("RBMQ_URL").ok();
    let (queue, consumer_handler) = queue::start(rbmq_url.as_deref(), pool.clone())
        .await
        .unwrap_or_else(|err| {
            warn!("Failed to start queue: {:?}", err);
            exit(1);
        });

    let app = routes::build(queue, pool);

    let port = "0.0.0.0:5000";
    let listener = tokio::net::TcpListener::bind(port).await.unwrap();

    let api_handler = tokio::spawn(async move {
        info!(" Server is starting on: {:?}", port);
        axum::serve(listener, app).await.unwrap();
    });

    tokio::select! {
        _ = consumer_handler => {
            warn!("Consumer handler kaboom!!!");
            exit(1);
        }
        _ = api_handler => {
            warn!("Api handler kaboom!!!");
            exit(1);
        }
    }
}
