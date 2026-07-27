use axum::Json;
use judge_ma_di::error::AppError;
use judge_ma_di::queue::Queue;
use judge_ma_di::routes::submission::{create_submission, CreateSubmission};
use std::sync::{Arc, Mutex};

type PublishedCall = (String, String, u64, String, String);

#[derive(Clone, Default)]
struct FakeQueue {
    published: Arc<Mutex<Vec<PublishedCall>>>,
}

impl Queue for FakeQueue {
    async fn publish(
        &self,
        routing_key: &str,
        task_id: String,
        submission_id: u64,
        code: String,
        language: String,
    ) -> anyhow::Result<()> {
        self.published.lock().unwrap().push((
            routing_key.to_string(),
            task_id,
            submission_id,
            code,
            language,
        ));
        Ok(())
    }
}

#[tokio::test]
async fn publishes_the_submission_to_the_queue() {
    let queue = FakeQueue::default();
    let req = CreateSubmission {
        task_id: "a_plus_b".to_string(),
        submission_id: 42,
        code: "int main() {}".to_string(),
        language: "cpp".to_string(),
    };

    let response = create_submission(Json(req), queue.clone()).await;

    assert!(response.is_ok());
    let published = queue.published.lock().unwrap();
    assert_eq!(published.len(), 1);
    assert_eq!(published[0].2, 42);
}

#[derive(Clone, Default)]
struct FailingQueue;

impl Queue for FailingQueue {
    async fn publish(
        &self,
        _routing_key: &str,
        _task_id: String,
        _submission_id: u64,
        _code: String,
        _language: String,
    ) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("queue is down"))
    }
}

#[tokio::test]
async fn returns_an_error_when_the_queue_publish_fails() {
    let req = CreateSubmission {
        task_id: "a_plus_b".to_string(),
        submission_id: 42,
        code: "int main() {}".to_string(),
        language: "cpp".to_string(),
    };

    let response = create_submission(Json(req), FailingQueue).await;

    assert!(matches!(response, Err(AppError::Internal(_))));
}
