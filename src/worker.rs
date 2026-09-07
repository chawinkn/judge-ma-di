use anyhow::{anyhow, Result};
use brotli::Decompressor;
use deadpool_postgres::{Client, Pool};
use std::io::Read;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info};

use crate::judge::runner::run;

fn poll_interval() -> Duration {
    Duration::from_millis(
        std::env::var("POLL_INTERVAL_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000),
    )
}

#[derive(Debug, PartialEq, Eq)]
pub struct PolledSubmission {
    pub submission_id: u64,
    pub task_id: String,
    pub code: Vec<u8>,
    pub language: String,
}

pub async fn run_worker(pool: Pool) -> Result<()> {
    info!("Polling for queued submissions");
    let poll_interval = poll_interval();

    loop {
        let db_client = pool.get().await?;

        match poll_next_submission(&db_client).await? {
            Some(polled) => judge_and_writeback(&db_client, polled).await?,
            None => {
                drop(db_client);
                sleep(poll_interval).await;
            }
        }
    }
}

// TODO: crash mid-judge leaves row stuck at 'Judging' forever. Need
// updated_at column + reclaim query.
pub async fn poll_next_submission(db_client: &Client) -> Result<Option<PolledSubmission>> {
    let row = db_client
        .query_opt(
            "
            UPDATE submission SET status = 'Judging'
            WHERE id = (
                SELECT id FROM submission
                WHERE status = 'In Queue'
                ORDER BY submitted_at ASC, id ASC
                LIMIT 1
                FOR UPDATE SKIP LOCKED
            )
            RETURNING id, task_id, code, language
            ",
            &[],
        )
        .await?;

    Ok(row.map(|row| PolledSubmission {
        submission_id: row.get::<_, i32>("id") as u64,
        task_id: row.get("task_id"),
        code: row.get("code"),
        language: row.get("language"),
    }))
}

/// `code` is brotli-compressed JSON. Frontend types it as `string[]` but
/// real rows store a bare string - both accepted.
pub fn decode_source_code(compressed: &[u8]) -> Result<String> {
    let mut decompressed = Vec::new();
    Decompressor::new(compressed, 4096).read_to_end(&mut decompressed)?;

    match serde_json::from_slice(&decompressed)? {
        serde_json::Value::String(code) => Ok(code),
        serde_json::Value::Array(files) => files
            .into_iter()
            .next()
            .and_then(|v| v.as_str().map(str::to_string))
            .ok_or_else(|| anyhow!("submission code array is empty or not a string")),
        other => Err(anyhow!(
            "submission code is neither a string nor an array: {other}"
        )),
    }
}

/// Shared writeback path so success/failure/decode-failure can't drift apart.
async fn judge_and_writeback(db_client: &Client, polled: PolledSubmission) -> Result<()> {
    let PolledSubmission {
        submission_id,
        task_id,
        code,
        language,
    } = polled;

    info!(
        submission_id,
        task_id = %task_id,
        "Start"
    );

    let attempt = match decode_source_code(&code) {
        Ok(source) => run(task_id.clone(), submission_id, source, language).await,
        Err(err) => Err(err),
    };

    match attempt {
        Ok(judge_result) => {
            info!(
                submission_id,
                task_id = %task_id,
                status = %judge_result.status,
                "Finished"
            );
            let result_json = serde_json::to_value(&judge_result.result).unwrap();
            debug!(
                submission_id,
                result = ?judge_result.result,
                "Detailed testcase results"
            );
            db_client.query_opt(
                     "UPDATE submission SET status = $1, score = $2, time = $3, memory = $4, result = $5 WHERE id = $6 AND status = 'Judging'",
                     &[
                         &judge_result.status.to_string(),
                         &(judge_result.score as i32),
                         &(judge_result.time as i32),
                        &(judge_result.memory as i32),
                        &result_json,
                        &(submission_id as i32),
                     ]
                 ).await?;
        }
        Err(err) => {
            error!(
                submission_id,
                task_id = %task_id,
                error = %err,
                "Submission judge error"
            );
            db_client
                .query_opt(
                    "UPDATE submission SET status = 'Judge Error' WHERE id = $1 AND status = 'Judging'",
                    &[&(submission_id as i32)],
                )
                .await?;
        }
    }

    Ok(())
}
