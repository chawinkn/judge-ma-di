use judge_ma_di::error::AppError;
use judge_ma_di::judge::languages::get_language;
use judge_ma_di::judge::task::{task_dir, validate_checker, Manifest, Task, ALLOWED_CHECKERS};
use std::path::{Path, PathBuf};

struct TempRoot(PathBuf);

impl TempRoot {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("judge_task_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write_task(&self, id: &str, manifest: &str, testcases: &[u64]) {
        let dir = self.0.join(id);
        std::fs::create_dir_all(dir.join("testcases")).unwrap();
        std::fs::write(dir.join("manifest.json"), manifest).unwrap();
        for n in testcases {
            std::fs::write(dir.join(format!("testcases/{n}.in")), "").unwrap();
            std::fs::write(dir.join(format!("testcases/{n}.sol")), "").unwrap();
        }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn manifest_json(checker: &str, num_testcases: u64, subtasks: &str) -> String {
    format!(
        r#"{{
        "time_limit": 1.0,
        "memory_limit": 256,
        "checker": "{checker}",
        "skip": false,
        "full_score": 100,
        "num_testcases": {num_testcases},
        "subtasks": {subtasks}
    }}"#
    )
}

#[test]
fn finds_known_languages() {
    let cpp = get_language("cpp").unwrap();
    assert_eq!(cpp.name(), "cpp");
    assert_eq!(cpp.ext(), "cpp");

    let c = get_language("c").unwrap();
    assert_eq!(c.name(), "c");
    assert_eq!(c.ext(), "c");

    let py = get_language("python").unwrap();
    assert_eq!(py.name(), "python");
    assert_eq!(py.ext(), "py");
}

#[test]
fn rejects_an_unknown_language() {
    let err = get_language("brainfuck").unwrap_err();
    assert!(matches!(err, AppError::BadRequest(_)));
}

#[test]
fn finds_a_known_task() {
    let task = Task::open("a_plus_b").unwrap();
    assert_eq!(task.manifest().num_testcases, 10);
    assert_eq!(task.manifest().testcase_count(), 10);
}

#[test]
fn rejects_an_unknown_task() {
    let err = Task::open("does_not_exist").err().unwrap();
    assert!(matches!(err, AppError::NotFound(_)));
}

#[test]
fn parses_skip_and_subtasks() {
    // tasks/0 is the only fixture with skip:true and non-empty subtasks.
    let task = Task::open("0").unwrap();
    let manifest = task.manifest();

    assert!(manifest.skip);
    assert_eq!(manifest.subtasks.len(), 3);
    assert_eq!(manifest.subtasks[0].full_score, 20);
    assert_eq!(manifest.subtasks[0].num_testcases, 2);
    assert_eq!(
        manifest.subtasks.iter().map(|s| s.full_score).sum::<u64>(),
        manifest.full_score
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
    let res = Manifest::parse(manifest_json("../../../bin/sh", 1, "[]").as_bytes());
    assert!(matches!(res.unwrap_err(), AppError::BadRequest(_)));
}

#[test]
fn rejects_malformed_manifest() {
    let res = Manifest::parse(b"{ not json");
    assert!(matches!(res.unwrap_err(), AppError::BadRequest(_)));
}

#[test]
fn rejects_manifest_with_zero_testcases() {
    let res = Manifest::parse(manifest_json("lcmp", 0, "[]").as_bytes());
    assert!(matches!(res.unwrap_err(), AppError::BadRequest(_)));
}

#[test]
fn rejects_manifest_with_zero_testcases_in_subtask() {
    let subtasks = r#"[
        { "full_score": 50, "num_testcases": 2 },
        { "full_score": 50, "num_testcases": 0 }
    ]"#;
    let res = Manifest::parse(manifest_json("lcmp", 2, subtasks).as_bytes());
    assert!(matches!(res.unwrap_err(), AppError::BadRequest(_)));
}

#[test]
fn accepts_subtask_manifest_with_zero_top_level_count_and_sums_subtasks() {
    let subtasks = r#"[
        { "full_score": 50, "num_testcases": 2 },
        { "full_score": 50, "num_testcases": 3 }
    ]"#;
    let manifest = Manifest::parse(manifest_json("lcmp", 0, subtasks).as_bytes()).unwrap();
    assert_eq!(manifest.testcase_count(), 5);
}

#[test]
fn rejects_task_id_with_path_traversal() {
    assert!(matches!(
        Task::open("../../etc/passwd").err().unwrap(),
        AppError::BadRequest(_)
    ));
    assert!(matches!(
        task_dir("../etc").unwrap_err(),
        AppError::BadRequest(_)
    ));
}

#[test]
fn task_dir_accepts_valid_identifiers() {
    for id in ["a_plus_b", "task-123", "0", "TASK"] {
        assert_eq!(task_dir(id).unwrap(), Path::new("tasks").join(id));
    }
}

#[test]
fn task_dir_rejects_invalid_characters() {
    for id in ["", "../etc", "task/1", "task\\1", "task.name", "task name"] {
        assert!(task_dir(id).is_err(), "Expected '{id}' to be rejected");
    }
}

#[test]
fn testcase_returns_input_and_solution_paths() {
    let task = Task::open("a_plus_b").unwrap();
    let (input, sol) = task.testcase(3);
    assert_eq!(input, Path::new("tasks/a_plus_b/testcases/3.in"));
    assert_eq!(sol, Path::new("tasks/a_plus_b/testcases/3.sol"));
}

#[test]
fn has_all_testcases_passes_when_all_pairs_exist() {
    let root = TempRoot::new("all_pairs");
    root.write_task("t", &manifest_json("lcmp", 2, "[]"), &[1, 2]);
    assert!(Task::open_in(root.path(), "t").unwrap().has_all_testcases());
}

#[test]
fn has_all_testcases_fails_when_testcases_dir_missing() {
    let root = TempRoot::new("no_dir");
    root.write_task("t", &manifest_json("lcmp", 1, "[]"), &[]);
    std::fs::remove_dir_all(root.path().join("t/testcases")).unwrap();
    assert!(!Task::open_in(root.path(), "t").unwrap().has_all_testcases());
}

#[test]
fn has_all_testcases_fails_when_a_pair_is_incomplete() {
    let root = TempRoot::new("incomplete");
    root.write_task("t", &manifest_json("lcmp", 2, "[]"), &[1, 2]);
    std::fs::remove_file(root.path().join("t/testcases/2.sol")).unwrap();
    assert!(!Task::open_in(root.path(), "t").unwrap().has_all_testcases());
}

#[test]
fn has_all_testcases_counts_subtasks_when_present() {
    let root = TempRoot::new("subtasks");
    let subtasks = r#"[
        { "full_score": 50, "num_testcases": 1 },
        { "full_score": 50, "num_testcases": 2 }
    ]"#;
    root.write_task("t", &manifest_json("lcmp", 0, subtasks), &[1, 2]);
    assert!(!Task::open_in(root.path(), "t").unwrap().has_all_testcases());

    root.write_task("t", &manifest_json("lcmp", 0, subtasks), &[1, 2, 3]);
    assert!(Task::open_in(root.path(), "t").unwrap().has_all_testcases());
}

#[test]
fn open_in_reads_allowed_languages_default() {
    let root = TempRoot::new("langs");
    root.write_task("t", &manifest_json("lcmp", 1, "[]"), &[1]);
    let task = Task::open_in(root.path(), "t").unwrap();
    assert!(task.manifest().allowed_languages.is_empty());
}
