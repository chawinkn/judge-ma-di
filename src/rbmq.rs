use std::{ env, fs };

use lapin::{
    options::{ BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions },
    types::FieldTable,
    BasicProperties,
    Channel,
    Connection,
    ConnectionProperties,
};
use futures::StreamExt;
use log::{ info, warn };
use serde::{ Deserialize, Serialize };
use crate::{ helper::get_language_config, runner::run };
use anyhow::Result;

#[derive(Debug, Deserialize, Serialize)]
pub struct Payload {
    task_id: String,
    submission_id: u64,
    language: String,
}

pub async fn get_channel() -> Result<Channel> {
    let addr = "amqp://root:root@127.0.0.1:5672";
    let conn = Connection::connect(addr, ConnectionProperties::default()).await?;
    let channel = conn.create_channel().await?;

    Ok(channel)
}

pub async fn publish_message(
    channel: Channel,
    routing_key: String,
    task_id: String,
    submission_id: u64,
    language: String
) -> Result<()> {
    info!(" [x] Sent to {:?} {:?}", routing_key, submission_id);

    let submission_payload = Payload {
        task_id,
        submission_id,
        language,
    };
    let payload = serde_json::to_string(&submission_payload)?;

    channel.basic_publish(
        "",
        &routing_key,
        BasicPublishOptions::default(),
        payload.as_bytes(),
        BasicProperties::default()
    ).await?;

    Ok(())
}

pub async fn create_queue(channel: Channel, queue_name: String) -> Result<()> {
    info!(" [x] Created queue {:?}", queue_name);

    channel.queue_declare(
        &queue_name,
        QueueDeclareOptions::default(),
        FieldTable::default()
    ).await?;

    Ok(())
}

pub async fn create_consumer(
    channel: Channel,
    queue_name: String,
    consumer_id: u64,
    consumer_tag: String
) -> Result<()> {
    info!(" [x] Created consumer {:?}", consumer_tag);

    let mut consumer = channel.basic_consume(
        &queue_name,
        &consumer_tag,
        BasicConsumeOptions::default(),
        FieldTable::default()
    ).await?;

    info!(" [*] Waiting for messages {:?}", consumer_tag);
    while let Some(delivery) = consumer.next().await {
        if let Ok(delivery) = delivery {
            let data = std::str::from_utf8(&delivery.data)?;
            let payload: Payload = serde_json::from_str(data)?;

            let task_id = payload.task_id;
            let submission_id = payload.submission_id;
            let language = payload.language;
            let language_config = get_language_config(&language).unwrap();
            let result = run(consumer_id, task_id, submission_id, language).await;
            match result {
                Ok(_) => {
                    let current_dir = env::current_dir()?;
                    let destination_path = current_dir
                        .join("temp")
                        .join(format!("{}.{}", submission_id, language_config.ext));
                    fs::remove_file(destination_path)?;
                }
                Err(_err) => {
                    warn!(" [x] {} {}", submission_id, _err);
                }
            }
            delivery.ack(BasicAckOptions::default()).await.expect("basic_ack");
        }
    }

    Ok(())
}
