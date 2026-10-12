use anyhow::{anyhow, Result};
use brotli::Decompressor;
use deadpool_postgres::{Client, Pool};
use std::io::Read;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info};

use crate::judge::runner::run;
use crate::queue::{self, Claimed};

pub async fn run_worker(pool: Pool) -> Result<()> {
    info!("Polling for queued submissions");
    let poll_interval = Duration::from_millis(
        std::env::var("POLL_INTERVAL_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000),
    );

    loop {
        let db_client = match pool.get().await {
            Ok(client) => client,
            Err(err) => {
                error!(error = %err, "Failed to get DB connection from pool, retrying");
                sleep(poll_interval).await;
                continue;
            }
        };

        match queue::claim(&db_client).await {
            Ok(Some(claimed)) => {
                if let Err(err) = judge_and_writeback(&db_client, claimed).await {
                    error!(error = %err, "Failed to judge or write back submission");
                }
            }
            Ok(None) => {
                sleep(poll_interval).await;
            }
            Err(err) => {
                error!(error = %err, "Failed to poll queued submissions, retrying");
                sleep(poll_interval).await;
            }
        }
    }
}

const MAX_DECOMPRESSED_SOURCE: u64 = 10 * 1024 * 1024; // 10 MB

/// `code` is brotli-compressed JSON. Frontend types it as `string[]` but
/// real rows store a bare string - both accepted.
pub fn decode_source_code(compressed: &[u8]) -> Result<String> {
    let mut decompressed = Vec::new();
    Decompressor::new(compressed, 4096)
        .take(MAX_DECOMPRESSED_SOURCE + 1)
        .read_to_end(&mut decompressed)?;

    if decompressed.len() as u64 > MAX_DECOMPRESSED_SOURCE {
        return Err(anyhow!("submission code exceeds maximum size of 10 MB"));
    }

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

async fn judge_and_writeback(db_client: &Client, claimed: Claimed) -> Result<()> {
    let Claimed {
        submission_id,
        task_id,
        code,
        language,
    } = claimed;

    info!(
        submission_id,
        task_id = %task_id,
        "Start"
    );

    let task_id_for_task = task_id.clone();
    let attempt = tokio::task::spawn_blocking(move || {
        decode_source_code(&code)
            .and_then(|source| run(&task_id_for_task, submission_id, source, &language))
    })
    .await
    .unwrap_or_else(|e| Err(anyhow::anyhow!("Judge task panicked: {e}")));

    match attempt {
        Ok(judge_result) => {
            info!(
                submission_id,
                task_id = %task_id,
                status = %judge_result.status,
                "Finished"
            );
            debug!(
                submission_id,
                result = ?judge_result.result,
                "Detailed testcase results"
            );
            queue::complete(db_client, submission_id, &judge_result).await
        }
        Err(err) => {
            error!(
                submission_id,
                task_id = %task_id,
                error = %err,
                "Submission judge error"
            );
            queue::fail(db_client, submission_id).await
        }
    }
}
