use judge_ma_di::judge::runner::{is_testcases_error, run, JudgeStatus};
use std::path::Path;

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

#[tokio::test]
async fn errors_on_unsupported_language() {
    let result = run(
        "a_plus_b".to_string(),
        1,
        "fn main() {}".to_string(),
        "unsupported_lang".to_string(),
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn errors_on_nonexistent_task() {
    let result = run(
        "nonexistent_task_xyz".to_string(),
        1,
        "int main() {}".to_string(),
        "cpp".to_string(),
    )
    .await;

    assert!(result.is_err());
}

#[test]
fn testcases_validation_passes_when_all_pairs_exist() {
    assert!(!is_testcases_error(
        Path::new("tasks/a_plus_b/testcases"),
        2
    ));
}

#[test]
fn testcases_validation_fails_when_testcases_dir_missing() {
    assert!(is_testcases_error(
        Path::new("tasks/_test_missing_testcases/testcases"),
        1
    ));
}

#[test]
fn testcases_validation_fails_when_testcase_count_insufficient() {
    assert!(is_testcases_error(
        Path::new("tasks/a_plus_b/testcases"),
        99
    ));
}
