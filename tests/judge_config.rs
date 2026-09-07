use judge_ma_di::error::AppError;
use judge_ma_di::judge::config::{
    get_language_config, get_task_config, validate_checker, ALLOWED_CHECKERS,
};

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

#[test]
fn validates_all_allowed_checkers() {
    for checker in ALLOWED_CHECKERS {
        assert!(validate_checker(checker).is_ok(), "Failed for {checker}");
    }
}

#[test]
fn rejects_invalid_and_malicious_checkers() {
    let malicious = [
        "../../../bin/sh",
        "../checker/lcmp",
        "checker/lcmp",
        "/bin/sh",
        "/usr/bin/python3",
        "sh",
        "bash",
        "custom",
        "evil",
        "lcmp\0evil",
        "lcmp; rm -rf /",
        "lcmp ",
        " lcmp",
        "",
    ];
    for checker in malicious {
        assert!(
            validate_checker(checker).is_err(),
            "Expected checker '{checker}' to be rejected"
        );
    }
}

#[test]
fn rejects_manifest_with_invalid_checker() {
    let temp_task_dir = std::path::Path::new("tasks/_test_bad_checker_fixture");
    let _ = std::fs::create_dir_all(temp_task_dir);
    let manifest_content = r#"{
        "time_limit": 1.0,
        "memory_limit": 256,
        "checker": "../../../bin/sh",
        "skip": false,
        "full_score": 100,
        "num_testcases": 1,
        "subtasks": []
    }"#;
    std::fs::write(temp_task_dir.join("manifest.json"), manifest_content).unwrap();

    let res = get_task_config("_test_bad_checker_fixture");
    let _ = std::fs::remove_dir_all(temp_task_dir);

    assert!(res.is_err());
    assert!(matches!(res.unwrap_err(), AppError::BadRequest(_)));
}
