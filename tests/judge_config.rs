use judge_ma_di::error::AppError;
use judge_ma_di::judge::config::{get_language_config, get_task_config};

#[test]
fn finds_a_known_language() {
    let config = get_language_config("cpp").unwrap();
    assert_eq!(config.lang, "cpp");
}

#[test]
fn rejects_an_unknown_language() {
    let err = get_language_config("brainfuck").unwrap_err();
    assert!(matches!(err, AppError::BadRequest(_)));
}

#[test]
fn finds_a_known_task() {
    let config = get_task_config("a_plus_b").unwrap();
    assert_eq!(config.num_testcases, 10);
}

#[test]
fn rejects_an_unknown_task() {
    let err = get_task_config("does_not_exist").unwrap_err();
    assert!(matches!(err, AppError::NotFound(_)));
}

#[test]
fn parses_skip_and_subtasks() {
    // tasks/0 is the only fixture with skip:true and non-empty subtasks.
    let config = get_task_config("0").unwrap();

    assert!(config.skip);
    assert_eq!(config.subtasks.len(), 3);
    assert_eq!(config.subtasks[0].full_score, 20);
    assert_eq!(config.subtasks[0].num_testcases, 2);
    assert_eq!(
        config.subtasks.iter().map(|s| s.full_score).sum::<u64>(),
        config.full_score
    );
}
