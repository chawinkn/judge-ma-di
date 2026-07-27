use judge_ma_di::judge::runner::{run, JudgeStatus};

// tasks/_test_missing_testcases has a manifest.json but no testcases/ dir,
// so run() should short-circuit before ever touching isolate.
#[tokio::test]
async fn reports_testcases_error_when_testcases_are_missing() {
    let result = run(
        "_test_missing_testcases".to_string(),
        1,
        "int main() {}".to_string(),
        "cpp".to_string(),
    )
    .await
    .unwrap();

    assert_eq!(result.status, JudgeStatus::TestcasesError);
}
