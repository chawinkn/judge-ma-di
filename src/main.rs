use std::{ sync::Arc, process::exit };
use axum::{ routing::{ get, post }, Router };
use lapin::Channel;
use tokio::spawn;
use log::{ info, warn };

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
    tracing_subscriber::fmt::init();

    info!(" Starting...");

    let consumer_channel = rbmq::get_channel().await.expect("Unable to create RabbitMQ channel");
    let shared_state = Arc::new(AppState { channel: consumer_channel.clone() });

    rbmq::create_queue(consumer_channel.clone(), "queue".to_string()).await.expect(
        "Unable to create RabbitMQ queue"
    );

    let judge_config = get_judge_config().unwrap();
    let max_concurrent_workers = judge_config.max_worker;

    let mut join_handles = Vec::new();

    for i in 0..max_concurrent_workers {
        let channel = consumer_channel.clone();
        let join_handle = spawn(async move {
            rbmq::create_consumer(
                channel,
                "queue".to_string(),
                i,
                format!("consumer {}", i)
            ).await.expect("Unable to create RabbitMQ consumer {}");
        });
        join_handles.push(join_handle);
    }

    let consumer_handler = spawn(async move {
        for handle in join_handles {
            let _ = handle.await;
        }
    });

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
        );

    let port = "0.0.0.0:3000";
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
