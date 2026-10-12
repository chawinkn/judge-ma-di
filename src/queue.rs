use anyhow::{Context, Result};
use deadpool_postgres::Client;

use crate::judge::runner::JudgeResult;

#[derive(Debug, PartialEq, Eq)]
pub struct Claimed {
    pub submission_id: u64,
    pub task_id: String,
    pub code: Vec<u8>,
    pub language: String,
}

// TODO: crash mid-judge leaves row stuck at 'Judging' forever. Need
// updated_at column + reclaim query.
pub async fn claim(db: &Client) -> Result<Option<Claimed>> {
    let row = db
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

    row.map(|row| {
        Ok(Claimed {
            submission_id: u64::try_from(row.get::<_, i32>("id"))?,
            task_id: row.get("task_id"),
            code: row.get("code"),
            language: row.get("language"),
        })
    })
    .transpose()
}

pub async fn complete(db: &Client, submission_id: u64, result: &JudgeResult) -> Result<()> {
    let result_json = serde_json::to_value(&result.result)?;
    db.execute(
        "UPDATE submission SET status = $1, score = $2, time = $3, memory = $4, result = $5 WHERE id = $6 AND status = 'Judging'",
        &[
            &result.status.to_string(),
            &i32::try_from(result.score).context("score overflows i32")?,
            &i32::try_from(result.time).context("time overflows i32")?,
            &i32::try_from(result.memory).context("memory overflows i32")?,
            &result_json,
            &i32::try_from(submission_id).context("submission id overflows i32")?,
        ],
    )
    .await?;
    Ok(())
}

pub async fn fail(db: &Client, submission_id: u64) -> Result<()> {
    db.execute(
        "UPDATE submission SET status = 'Judge Error' WHERE id = $1 AND status = 'Judging'",
        &[&i32::try_from(submission_id).context("submission id overflows i32")?],
    )
    .await?;
    Ok(())
}
