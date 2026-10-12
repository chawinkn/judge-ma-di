use judge_ma_di::judge::runner::{run, JudgeStatus};
use judge_ma_di::judge::sandbox::check_output;
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
fn check_output_rejects_malicious_checker() {
    let res = check_output(
        "../../../bin/sh",
        Path::new("tasks/a_plus_b/testcases/1.in"),
        Path::new("tasks/a_plus_b/testcases/1.sol"),
        Path::new("tasks/a_plus_b/testcases/1.sol"),
    );
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
fn check_output_evaluates_correct_and_wrong_output() {
    let temp_dir = std::env::temp_dir().join("judge_test_check_dir");
    let _ = std::fs::create_dir_all(&temp_dir);
    let input = Path::new("tasks/a_plus_b/testcases/1.in");
    let sol = Path::new("tasks/a_plus_b/testcases/1.sol");
    let out = temp_dir.join("out.out");

    std::fs::copy(sol, &out).unwrap();
    assert!(check_output("lcmp", input, &out, sol).unwrap());

    std::fs::write(&out, b"WRONG_ANSWER_12345\n").unwrap();
    assert!(!check_output("lcmp", input, &out, sol).unwrap());

    let _ = std::fs::remove_dir_all(&temp_dir);
}
