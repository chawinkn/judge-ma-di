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
use tokio_postgres::Client;
use crate::runner::run;
use anyhow::Result;
use crate::Arc;

#[derive(Debug, Deserialize, Serialize)]
pub struct Payload {
    task_id: String,
    submission_id: u64,
    code: String,
    language: String,
}

pub async fn get_channel(rmbq_url: &str) -> Result<Channel> {
    let conn = Connection::connect(rmbq_url, ConnectionProperties::default()).await?;
    let channel = conn.create_channel().await?;

    Ok(channel)
}

pub async fn publish_message(
    channel: Channel,
    routing_key: String,
    task_id: String,
    submission_id: u64,
    code: String,
    language: String
) -> Result<()> {
    info!(" [x] Sent to {:?} {:?}", routing_key, submission_id);

    let submission_payload = Payload {
        task_id,
        submission_id,
        code,
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
    db_client: Arc<Client>,
    queue_name: String,
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
            let code = payload.code;

            info!(" [*] Judging {}", submission_id);

            let row = db_client.query_opt(
                "SELECT id FROM submission WHERE id = $1",
                &[&(submission_id as i32)]
            ).await?;

            if row.is_none() {
                warn!(" [x] Submission ID {} not found", submission_id);
                delivery.ack(BasicAckOptions::default()).await?;
                continue;
            }

            db_client.query_opt(
                "UPDATE submission SET status = $1 WHERE id = $2",
                &[&"Judging", &(submission_id as i32)]
            ).await?;

            let result = run(task_id, submission_id, code, language).await;

            match result {
                Ok(judge_result) => {
                    info!(" [x] {} Finished", submission_id);
                    let data = serde_json::to_value(&judge_result.result).unwrap();
                    db_client.query_opt(
                        "UPDATE submission SET status = $1, score = $2, time = $3, memory = $4, result = $5 WHERE id = $6",
                        &[
                            &judge_result.status,
                            &(judge_result.score as i32),
                            &(judge_result.time as i32),
                            &(judge_result.memory as i32),
                            &data,
                            &(submission_id as i32),
                        ]
                    ).await?;
                }
                Err(_err) => {
                    warn!(" [x] {} {}", submission_id, _err);
                    db_client.query_opt(
                        "UPDATE submission SET status = 'Judge Error' WHERE id = $1",
                        &[&(submission_id as i32)]
                    ).await?;
                }
            }
            delivery.ack(BasicAckOptions::default()).await?;
        }
    }

    Ok(())
}
