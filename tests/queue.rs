use judge_ma_di::queue::{JobQueue, Queue};
use tokio::sync::mpsc;

#[tokio::test]
async fn in_memory_backend_round_trips_a_payload() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let queue = JobQueue::InMemory(tx);

    queue
        .publish(
            "queue",
            "a_plus_b".to_string(),
            42,
            "int main() {}".to_string(),
            "cpp".to_string(),
        )
        .await
        .unwrap();

    let payload = rx.recv().await.unwrap();
    assert_eq!(payload.task_id, "a_plus_b");
    assert_eq!(payload.submission_id, 42);
    assert_eq!(payload.language, "cpp");
}

#[tokio::test]
async fn in_memory_backend_preserves_publish_order() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let queue = JobQueue::InMemory(tx);

    for submission_id in 1..=3 {
        queue
            .publish(
                "queue",
                "a_plus_b".to_string(),
                submission_id,
                "int main() {}".to_string(),
                "cpp".to_string(),
            )
            .await
            .unwrap();
    }

    for expected_id in 1..=3 {
        assert_eq!(rx.recv().await.unwrap().submission_id, expected_id);
    }
}
