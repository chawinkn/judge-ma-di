use std::{ sync::Arc, process::exit, time::Duration };
use axum::{
    error_handling::HandleErrorLayer,
    http::StatusCode,
    extract::DefaultBodyLimit,
    routing::{ delete, get, post },
    Router,
};
use lapin::Channel;
use tokio::{ spawn, time::interval };
use log::{ info, warn };
use tokio::time::sleep;
use tokio_postgres::Client;
use dotenv::dotenv;
use postgres_openssl::MakeTlsConnector;
use openssl::ssl::{ SslConnector, SslMethod };
use tower_http::cors::{ Any, CorsLayer };
use axum_token_auth::{ AuthConfig, TokenConfig };

pub mod routes;
pub mod helper;
pub mod rbmq;
pub mod isolate;
pub mod runner;

pub struct AppState {
    channel: Channel,
}

async fn handle_auth_error(err: tower::BoxError) -> (StatusCode, &'static str) {
    match err.downcast::<axum_token_auth::ValidationErrors>() {
        Ok(_) => { (StatusCode::UNAUTHORIZED, "Request is not authorized") }
        Err(_) => { (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error") }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt::init();

    info!(" Starting...");

    let persistent_secret = cookie::Key::generate();

    let token = TokenConfig::new_token("token");
    let cfg = AuthConfig {
        token_config: Some(token.clone()),
        persistent_secret,
        ..Default::default()
    };

    let auth_layer = cfg.clone().into_layer();

    let postgres_url = std::env::var("POSTGRES_URL").expect("POSTGRES_URL not found");

    let builder = SslConnector::builder(SslMethod::tls()).unwrap();
    let mut connector = MakeTlsConnector::new(builder.build());
    connector.set_callback(|config, _| {
        config.set_verify_hostname(false);
        Ok(())
    });

    let (pg_client, connection) = tokio_postgres
        ::connect(&postgres_url, connector).await
        .expect("Unable to connect to PostgreSQL");
    let client: Arc<Client> = Arc::new(pg_client);

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            warn!("Error connecting to PostgreSQL: {}", e);
            exit(1);
        }
    });

    let db_client = client.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(240));
        loop {
            interval.tick().await;
            if let Err(e) = db_client.simple_query("SELECT 1").await {
                warn!("Failed to execute heartbeat query: {:?}", e);
                exit(1);
            }
        }
    });

    let rbmq_url = std::env::var("RBMQ_URL").expect("RBMQ_URL not found");

    let max_retries = 5;
    let mut retries = 0;

    let consumer_channel = loop {
        match rbmq::get_channel(&rbmq_url).await {
            Ok(channel) => {
                break channel;
            }
            Err(err) => {
                warn!("Failed to create RabbitMQ channel: {:?}", err);
                retries += 1;
                if retries >= max_retries {
                    warn!("Reached maximum retry limit. Exiting...");
                    exit(1);
                }
                sleep(Duration::from_secs(5)).await;
            }
        }
    };

    let shared_state = Arc::new(AppState {
        channel: consumer_channel.clone(),
    });

    loop {
        match rbmq::create_queue(consumer_channel.clone(), "queue".to_string()).await {
            Ok(_) => {
                break;
            }
            Err(err) => {
                warn!("Failed to create RabbitMQ queue: {:?}", err);
                sleep(Duration::from_secs(5)).await;
            }
        }
    }

    let max_worker_env = std::env::var("MAX_WORKER").expect("MAX_WORKER not found");
    let max_worker = match max_worker_env.parse::<i32>() {
        Ok(num) => num,
        Err(_) => {
            warn!("MAX_WORKER must be integer");
            0
        }
    };

    let mut join_handles = Vec::new();

    for i in 0..max_worker {
        let channel = consumer_channel.clone();
        let db_client = client.clone();
        let join_handle = spawn(async move {
            rbmq::create_consumer(
                channel,
                db_client,
                "queue".to_string(),
                format!("consumer {}", i)
            ).await.expect("Unable to create RabbitMQ consumer");
        });
        join_handles.push(join_handle);
    }

    let consumer_handler = spawn(async move {
        for handle in join_handles {
            let _ = handle.await;
        }
    });

    let cors = CorsLayer::new().allow_headers(Any).allow_methods(Any).allow_origin(Any);

    let app = Router::new()
        .route("/api/healthchecker", get(routes::healthchecker::health_checker))
        .route(
            "/api/submit",
            post({
                let shared_state = Arc::clone(&shared_state);
                move |body| routes::submission::create_submission(body, shared_state)
            })
        )
        .route(
            "/api/task/:id",
            get({
                let shared_state = Arc::clone(&shared_state);
                move |path| routes::task::get_task_testcases(path, shared_state)
            })
        )
        .route(
            "/api/task/:id",
            post(routes::task::upload_task).layer(DefaultBodyLimit::max(1024 * 1000 * 10))
        )
        .route("/api/task/:id", delete(routes::task::delete_task))
        .route(
            "/api/desc/:id",
            get({
                let shared_state = Arc::clone(&shared_state);
                move |path| routes::desc::get_desc(path, shared_state)
            })
        )
        .route(
            "/api/manifest/:id",
            get({
                let shared_state = Arc::clone(&shared_state);
                move |path| routes::manifest::get_manifest(path, shared_state)
            })
        )
        .layer(cors)
        .layer(
            tower::ServiceBuilder
                ::new()
                .layer(HandleErrorLayer::new(handle_auth_error))
                .layer(auth_layer)
        );

    let port = "0.0.0.0:5000";
    let listener = tokio::net::TcpListener::bind(port).await.unwrap();

    let api_handler = spawn(async move { axum::serve(listener, app).await.unwrap() });

    info!(" Server is starting on: {:?}", port);

    info!(" Token: {}", token.value);

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
