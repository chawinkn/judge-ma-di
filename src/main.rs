use std::{ sync::Arc, process::exit, time::Duration };
use axum::{ extract::multipart, routing::{ delete, get, post }, Router };
use lapin::Channel;
use tokio::{ spawn, time::interval };
use log::{ info, warn };
use tokio::time::sleep;
use tokio_postgres::{ Client, Config };
use dotenv::dotenv;
use postgres_openssl::MakeTlsConnector;
use openssl::ssl::{ SslConnector, SslMethod };
use tower_http::cors::{ Any, CorsLayer };

use crate::helper::get_judge_config;
pub mod routes;
pub mod helper;
pub mod rbmq;
pub mod isolate;
pub mod runner;

pub struct AppState {
    channel: Channel,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt::init();

    info!(" Starting...");

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
            panic!("Error connecting to PostgreSQL: {}", e);
        }
    });

    let db_client = client.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(300));
        loop {
            interval.tick().await;
            if let Err(e) = db_client.simple_query("SELECT 1").await {
                warn!("Failed to execute heartbeat query: {:?}", e);
            }
        }
    });

    let consumer_channel = loop {
        match rbmq::get_channel().await {
            Ok(channel) => {
                break channel;
            }
            Err(err) => {
                warn!("Failed to create RabbitMQ channel: {:?}", err);
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

    let judge_config = get_judge_config().unwrap();
    let max_concurrent_workers = judge_config.max_worker;

    let mut join_handles = Vec::new();

    for i in 0..max_concurrent_workers {
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

    // let origins = ["http://localhost:3000".parse().unwrap()];

    let cors = CorsLayer::new().allow_headers(Any).allow_methods(Any).allow_origin(Any);

    let app = Router::new()
        .route(
            "/",
            get(|| async { "OK" })
        )
        .route(
            "/submit",
            post({
                let shared_state = Arc::clone(&shared_state);
                move |body| routes::submission::create_submission(body, shared_state)
            })
        )
        .route(
            "/task/:id",
            get({
                let shared_state = Arc::clone(&shared_state);
                move |path| routes::task::get_task(path, shared_state)
            })
        )
        .route("/task/:id", post(routes::task::create_task))
        .route("/task/:id", delete(routes::task::delete_task))
        .route(
            "/desc/:id",
            get({
                let shared_state = Arc::clone(&shared_state);
                move |path| routes::desc::get_desc(path, shared_state)
            })
        )
        .layer(cors);

    let port = "0.0.0.0:5000";
    let listener = tokio::net::TcpListener::bind(port).await.unwrap();

    let api_handler = spawn(async move { axum::serve(listener, app).await.unwrap() });

    info!(" Server is starting on: {:?}", port);

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
