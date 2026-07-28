use anyhow::{Context, Result};
use deadpool_postgres::{Client, Pool};
use futures::StreamExt;
use lapin::{
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, BasicQosOptions,
        QueueDeclareOptions,
    },
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties,
};
use serde::{Deserialize, Serialize};
use std::process::exit;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use crate::judge::runner::run;

pub async fn start(rbmq_url: Option<&str>, pool: Pool) -> Result<(JobQueue, JoinHandle<()>)> {
    match rbmq_url {
        Some(rbmq_url) => {
            let consumer_channel = get_channel(rbmq_url)
                .await
                .context("Failed to create RabbitMQ channel")?;
            create_queue(consumer_channel.clone(), "queue")
                .await
                .context("Failed to create RabbitMQ queue")?;

            let publish_channel = consumer_channel.clone();

            let handle = tokio::spawn(async move {
                if let Err(err) = create_consumer(consumer_channel, pool, "queue", "Consumer").await
                {
                    warn!("Failed to create consumer: {:?}", err);
                    exit(1);
                }
            });

            Ok((JobQueue::RabbitMq(publish_channel), handle))
        }
        None => {
            info!("RBMQ_URL not set, using built-in in-process queue");
            let (tx, rx) = mpsc::unbounded_channel();

            let handle = tokio::spawn(async move {
                if let Err(err) = run_in_memory_consumer(rx, pool).await {
                    warn!("Failed to run in-memory consumer: {:?}", err);
                    exit(1);
                }
            });

            Ok((JobQueue::InMemory(tx), handle))
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Payload {
    pub task_id: String,
    pub submission_id: u64,
    pub code: String,
    pub language: String,
}

/// The one boundary likely to change (RabbitMQ -> something else later).
/// Handlers depend on this trait, not on `lapin::Channel` directly, so they
/// can be tested against a fake and the real queue can be swapped without
/// touching call sites.
pub trait Queue {
    fn publish(
        &self,
        routing_key: &str,
        task_id: String,
        submission_id: u64,
        code: String,
        language: String,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

#[derive(Clone)]
pub enum JobQueue {
    RabbitMq(Channel),
    InMemory(mpsc::UnboundedSender<Payload>),
}

impl Queue for JobQueue {
    async fn publish(
        &self,
        routing_key: &str,
        task_id: String,
        submission_id: u64,
        code: String,
        language: String,
    ) -> Result<()> {
        match self {
            JobQueue::RabbitMq(channel) => {
                let payload = serde_json::to_string(&Payload {
                    task_id,
                    submission_id,
                    code,
                    language,
                })?;

                channel
                    .basic_publish(
                        "",
                        routing_key,
                        BasicPublishOptions::default(),
                        payload.as_bytes(),
                        BasicProperties::default(),
                    )
                    .await?;

                Ok(())
            }
            JobQueue::InMemory(tx) => {
                tx.send(Payload {
                    task_id,
                    submission_id,
                    code,
                    language,
                })?;
                Ok(())
            }
        }
    }
}

async fn get_channel(rmbq_url: &str) -> Result<Channel> {
    let conn = Connection::connect(rmbq_url, ConnectionProperties::default()).await?;
    let channel = conn.create_channel().await?;

    Ok(channel)
}

async fn create_queue(channel: Channel, queue_name: &str) -> Result<()> {
    info!("Creating queue: {queue_name}");

    channel
        .queue_declare(
            queue_name,
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    Ok(())
}

async fn create_consumer(
    channel: Channel,
    pool: Pool,
    queue_name: &str,
    consumer_tag: &str,
) -> Result<()> {
    info!("Creating consumer: {consumer_tag}");

    // Fair dispatch across instances: don't let RabbitMQ hand this consumer
    // a second message until it acks the first.
    channel.basic_qos(1, BasicQosOptions::default()).await?;

    let mut consumer = channel
        .basic_consume(
            queue_name,
            consumer_tag,
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    let db_client = pool.get().await?;

    info!("Waiting for messages: {:?}", consumer_tag);

    while let Some(delivery) = consumer.next().await {
        let delivery = delivery?;

        let data = std::str::from_utf8(&delivery.data)?;
        let payload: Payload = serde_json::from_str(data)?;

        judge_payload(&db_client, payload).await?;
        delivery.ack(BasicAckOptions::default()).await?;
    }

    Ok(())
}

async fn run_in_memory_consumer(
    mut rx: mpsc::UnboundedReceiver<Payload>,
    pool: Pool,
) -> Result<()> {
    let db_client = pool.get().await?;

    info!("Waiting for messages: in-memory queue");

    while let Some(payload) = rx.recv().await {
        judge_payload(&db_client, payload).await?;
    }

    Ok(())
}

/// Shared by every consumer so they can't drift apart.
async fn judge_payload(db_client: &Client, payload: Payload) -> Result<()> {
    let Payload {
        task_id,
        submission_id,
        code,
        language,
    } = payload;

    info!("Judging submission_id: {}", submission_id);

    let row = db_client
        .query_opt(
            "UPDATE submission SET status = 'Judging' WHERE id = $1 AND status = 'In Queue' RETURNING id",
            &[&(submission_id as i32)],
        )
        .await?;

    if row.is_none() {
        warn!(
            "Submission ID {} not found or not in queue, skipping",
            submission_id
        );
        return Ok(());
    };

    match run(task_id, submission_id, code, language).await {
        Ok(judge_result) => {
            info!(
                "Finished submission_id: {}, status: {}, score: {}",
                submission_id, judge_result.status, judge_result.score
            );
            let data = serde_json::to_value(&judge_result.result).unwrap();
            info!("{:#?}", judge_result.result);
            db_client.query_opt(
                     "UPDATE submission SET status = $1, score = $2, time = $3, memory = $4, result = $5 WHERE id = $6",
                     &[
                         &judge_result.status.to_string(),
                         &(judge_result.score as i32),
                         &(judge_result.time as i32),
                        &(judge_result.memory as i32),
                        &data,
                        &(submission_id as i32),
                     ]
                 ).await?;
        }
        Err(err) => {
            error!("Error submission_id: {} {:#?}", submission_id, err);
            db_client
                .query_opt(
                    "UPDATE submission SET status = 'Judge Error' WHERE id = $1",
                    &[&(submission_id as i32)],
                )
                .await?;
        }
    }

    Ok(())
}
