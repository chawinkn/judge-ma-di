use judge_ma_di::judge::isolate::Isolate;
use judge_ma_di::judge::runner::{is_testcases_error, run, JudgeStatus};
use std::path::Path;

// tasks/_test_missing_testcases has a manifest.json but no testcases/ dir,
// so run() should short-circuit before ever touching isolate.
#[test]
fn reports_testcases_error_when_testcases_are_missing() {
    let result = run(
        "_test_missing_testcases",
        1,
        "int main() {}".to_string(),
        "cpp",
    )
    .unwrap();

    assert_eq!(result.status, JudgeStatus::TestcasesError);
}

#[test]
fn errors_on_unsupported_language() {
    let result = run(
        "a_plus_b",
        1,
        "fn main() {}".to_string(),
        "unsupported_lang",
    );

    assert!(result.is_err());
}

#[test]
fn errors_on_nonexistent_task() {
    let result = run(
        "nonexistent_task_xyz",
        1,
        "int main() {}".to_string(),
        "cpp",
    );

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

#[test]
fn testcases_validation_fails_when_testcase_count_is_zero() {
    assert!(is_testcases_error(Path::new("tasks/a_plus_b/testcases"), 0));
}

#[test]
fn isolate_check_rejects_malicious_checker() {
    let mut isolate = Isolate::default();
    isolate.checker = "../../../bin/sh".to_string();
    let res = isolate.check(1);
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("Unsupported checker"));
}

#[test]
fn runner_rejects_malicious_task_id() {
    let res = run("../../etc", 1, "int main() {}".to_string(), "cpp");
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("invalid task_id"));
}

#[test]
fn isolate_check_evaluates_correct_and_wrong_output() {
    let temp_dir = std::env::temp_dir().join("judge_test_check_dir");
    let _ = std::fs::create_dir_all(&temp_dir);

    let sol = std::fs::read("tasks/a_plus_b/testcases/1.sol").unwrap();
    std::fs::write(temp_dir.join("out.out"), &sol).unwrap();

    let mut isolate = Isolate::default();
    isolate.box_path = temp_dir.clone();
    isolate.checker = "lcmp".to_string();
    isolate.testcases_dir = Path::new("tasks/a_plus_b/testcases").to_path_buf();

    let res_correct = isolate.check(1);
    assert!(res_correct.unwrap());

    std::fs::write(temp_dir.join("out.out"), b"WRONG_ANSWER_12345\n").unwrap();
    let res_wrong = isolate.check(1);
    assert!(!res_wrong.unwrap());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn isolate_meta_path_is_outside_box() {
    let mut isolate = Isolate::default();
    isolate.box_id = 42;
    isolate.box_path = std::path::PathBuf::from("/var/local/lib/isolate/42/box");
    let meta = isolate.meta_path();
    assert_eq!(meta, std::env::temp_dir().join("isolate_meta_42.txt"));
    assert_ne!(meta.parent(), Some(isolate.box_path.as_path()));
}

#[test]
fn isolate_cleanup_is_idempotent() {
    let mut isolate = Isolate::default();
    // initialized is false by default -> cleanup is a no-op returning Ok(())
    assert!(isolate.cleanup().is_ok());
    assert!(!isolate.initialized);
    // calling cleanup again is still a no-op returning Ok(())
    assert!(isolate.cleanup().is_ok());
}
