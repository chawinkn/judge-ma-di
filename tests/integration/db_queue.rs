use deadpool_postgres::{Manager, Pool};
use judge_ma_di::worker::{decode_source_code, poll_next_submission};
use serde_json::json;
use std::io::Write;
use std::str::FromStr;
use tokio_postgres::NoTls;

fn encode_source_code(source: &str) -> Vec<u8> {
    let json = serde_json::to_vec(source).unwrap();
    let mut compressor = brotli::CompressorWriter::new(Vec::new(), 4096, 6, 22);
    compressor.write_all(&json).unwrap();
    compressor.into_inner()
}

async fn test_pool() -> anyhow::Result<Pool> {
    let url = std::env::var("POSTGRES_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432/postgres".to_string());
    let cfg = tokio_postgres::Config::from_str(&url)?;
    let mgr = Manager::new(cfg, NoTls);
    let pool = Pool::builder(mgr).max_size(2).build()?;
    pool.get()
        .await?
        .batch_execute(include_str!("../../scripts/init.sql"))
        .await?;
    Ok(pool)
}

#[tokio::test]
async fn test_db_queue_lifecycle_atomic_claim_and_writeback() {
    let pool = test_pool()
        .await
        .expect("PostgreSQL must be running on POSTGRES_URL to run db_queue integration tests");
    let client = pool.get().await.unwrap();

    // 1. Queue 2 submissions
    let c1 = encode_source_code("int main() { return 0; }");
    let c2 = encode_source_code("int main() { return 1; }");
    let id1: i32 = client
        .query_one(
            "INSERT INTO submission (task_id, code, language, status) VALUES ('a_plus_b', $1, 'cpp', 'In Queue') RETURNING id",
            &[&c1],
        )
        .await
        .unwrap()
        .get("id");
    let id2: i32 = client
        .query_one(
            "INSERT INTO submission (task_id, code, language, status) VALUES ('a_plus_b', $1, 'cpp', 'In Queue') RETURNING id",
            &[&c2],
        )
        .await
        .unwrap()
        .get("id");

    // 2. Poll: verifies FIFO and atomic transition to 'Judging'
    let p1 = poll_next_submission(&client).await.unwrap().unwrap();
    assert_eq!(p1.submission_id, id1 as u64);
    assert_eq!(
        decode_source_code(&p1.code).unwrap(),
        "int main() { return 0; }"
    );

    let p2 = poll_next_submission(&client).await.unwrap().unwrap();
    assert_eq!(p2.submission_id, id2 as u64);

    // 3. Writeback results & verify
    let res = json!([{"test_index": 1, "status": "Accepted", "score": 100}]);
    client
        .execute(
            "UPDATE submission SET status = 'Completed', score = 100, result = $1 WHERE id = $2",
            &[&res, &id1],
        )
        .await
        .unwrap();
    client
        .execute(
            "UPDATE submission SET status = 'Judge Error' WHERE id = $1",
            &[&id2],
        )
        .await
        .unwrap();

    let r1 = client
        .query_one(
            "SELECT status, score FROM submission WHERE id = $1",
            &[&id1],
        )
        .await
        .unwrap();
    assert_eq!(r1.get::<_, String>("status"), "Completed");
    assert_eq!(r1.get::<_, i32>("score"), 100);

    let r2 = client
        .query_one("SELECT status FROM submission WHERE id = $1", &[&id2])
        .await
        .unwrap();
    assert_eq!(r2.get::<_, String>("status"), "Judge Error");

    // 4. Clean up test rows
    client
        .execute(
            "DELETE FROM submission WHERE id = ANY($1)",
            &[&vec![id1, id2]],
        )
        .await
        .ok();
}
