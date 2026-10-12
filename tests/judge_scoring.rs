use judge_ma_di::judge::isolate::{IsolateResult, RunVerdict};
use judge_ma_di::judge::languages::CPP;
use judge_ma_di::judge::runner::{judge, JudgeStatus, RunStatus};
use judge_ma_di::judge::sandbox::Sandbox;
use judge_ma_di::judge::task::Task;
use std::collections::VecDeque;
use std::path::PathBuf;

struct TempRoot(PathBuf);

impl TempRoot {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("judge_scoring_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn task(&self, manifest_fields: &str, testcases: u64) -> Task {
        let dir = self.0.join("t");
        std::fs::create_dir_all(dir.join("testcases")).unwrap();
        std::fs::write(
            dir.join("manifest.json"),
            format!(
                r#"{{"time_limit": 1.0, "memory_limit": 256, "checker": "lcmp", {manifest_fields}}}"#
            ),
        )
        .unwrap();
        for n in 1..=testcases {
            std::fs::write(dir.join(format!("testcases/{n}.in")), "").unwrap();
            std::fs::write(dir.join(format!("testcases/{n}.sol")), "").unwrap();
        }
        Task::open_in(&self.0, "t").unwrap()
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn normal(full_score: u64, num_testcases: u64, root: &TempRoot) -> Task {
    root.task(
        &format!(
            r#""skip": false, "full_score": {full_score}, "num_testcases": {num_testcases}, "subtasks": []"#
        ),
        num_testcases,
    )
}

fn with_subtasks(skip: bool, subtasks: &[(u64, u64)], root: &TempRoot) -> Task {
    let list = subtasks
        .iter()
        .map(|(score, n)| format!(r#"{{"full_score": {score}, "num_testcases": {n}}}"#))
        .collect::<Vec<_>>()
        .join(",");
    root.task(
        &format!(r#""skip": {skip}, "full_score": 100, "num_testcases": 0, "subtasks": [{list}]"#),
        subtasks.iter().map(|(_, n)| n).sum(),
    )
}

// Scripted sandbox: run()/check() results are consumed in order, and panic
// if called more times than scripted - that's how we prove skip-on-failure
// (and the "no check() when the verdict isn't OK" branch) actually stop
// calling the sandbox instead of just producing a zero score some other way.
struct FakeSandbox {
    compiles: bool,
    compile_calls: u32,
    run_results: VecDeque<IsolateResult>,
    check_results: VecDeque<bool>,
}

impl FakeSandbox {
    fn with_verdicts(verdicts: impl IntoIterator<Item = RunVerdict>) -> Self {
        Self {
            compiles: true,
            compile_calls: 0,
            run_results: verdicts
                .into_iter()
                .map(|status| IsolateResult {
                    status,
                    ..Default::default()
                })
                .collect(),
            check_results: VecDeque::new(),
        }
    }

    fn checks(mut self, checks: impl IntoIterator<Item = bool>) -> Self {
        self.check_results = checks.into_iter().collect();
        self
    }
}

impl Sandbox for FakeSandbox {
    fn compile(&mut self) -> anyhow::Result<bool> {
        self.compile_calls += 1;
        Ok(self.compiles)
    }

    fn run(&mut self, _test_index: u64) -> anyhow::Result<IsolateResult> {
        Ok(self
            .run_results
            .pop_front()
            .expect("run() called more times than scripted"))
    }

    fn check(&mut self, _test_index: u64) -> anyhow::Result<bool> {
        Ok(self
            .check_results
            .pop_front()
            .expect("check() called more times than scripted"))
    }
}

#[test]
fn wrong_answer_scores_zero() {
    let root = TempRoot::new("wa");
    let task = normal(50, 1, &root);
    let mut sandbox = FakeSandbox::with_verdicts([RunVerdict::VerdictOK]).checks([false]);

    let result = judge(&task, &CPP, &mut sandbox).unwrap();

    assert_eq!(result.result[0].status, RunStatus::WrongAnswer);
    assert_eq!(result.result[0].score, 0);
    assert_eq!(result.score, 0);
}

#[test]
fn bad_verdict_skips_checker() {
    // No entries in check_results - a check() call here would panic, so this
    // also proves the checker is skipped entirely for a non-OK verdict.
    let root = TempRoot::new("tle");
    let task = normal(50, 1, &root);
    let mut sandbox = FakeSandbox::with_verdicts([RunVerdict::VerdictTLE]);

    let result = judge(&task, &CPP, &mut sandbox).unwrap();

    assert_eq!(
        result.result[0].status,
        RunStatus::Verdict(RunVerdict::VerdictTLE)
    );
    assert_eq!(result.result[0].score, 0);
}

#[test]
fn normal_partial_score() {
    // Unlike subtasks, flat scoring isn't all-or-nothing: testcase 1 and 4
    // keep their score even though 2 and 3 fail.
    let root = TempRoot::new("partial");
    let task = normal(100, 4, &root);
    let mut sandbox = FakeSandbox::with_verdicts([
        RunVerdict::VerdictOK,
        RunVerdict::VerdictOK,
        RunVerdict::VerdictTLE,
        RunVerdict::VerdictOK,
    ])
    .checks([true, false, true]);

    let result = judge(&task, &CPP, &mut sandbox).unwrap();

    assert_eq!(result.status, JudgeStatus::Completed);
    assert_eq!(result.result.len(), 4);
    assert_eq!(result.result[0].score, 25);
    assert_eq!(result.result[1].score, 0);
    assert_eq!(result.result[2].score, 0);
    assert_eq!(result.result[3].score, 25);
    assert_eq!(result.score, 50);
}

#[test]
fn normal_time_memory_max() {
    let root = TempRoot::new("max");
    let task = normal(100, 2, &root);
    let mut sandbox = FakeSandbox {
        run_results: VecDeque::from([
            IsolateResult {
                status: RunVerdict::VerdictOK,
                time_usage: 0.1,
                memory_usage: 1000,
            },
            IsolateResult {
                status: RunVerdict::VerdictOK,
                time_usage: 0.5,
                memory_usage: 500,
            },
        ]),
        check_results: VecDeque::from([true, true]),
        ..FakeSandbox::with_verdicts([])
    };

    let result = judge(&task, &CPP, &mut sandbox).unwrap();

    assert_eq!(result.time, 500); // 0.5s -> 500ms, the slower of the two
    assert_eq!(result.memory, 1000); // the larger of the two
}

#[test]
fn subtasks_skip_zeroes_rest() {
    let root = TempRoot::new("skip");
    let task = with_subtasks(true, &[(100, 3)], &root);
    let mut sandbox = FakeSandbox::with_verdicts([RunVerdict::VerdictOK, RunVerdict::VerdictOK])
        .checks([true, false]);

    let result = judge(&task, &CPP, &mut sandbox).unwrap();

    assert_eq!(result.result.len(), 3);
    assert_eq!(
        result.result[0].status,
        RunStatus::Verdict(RunVerdict::VerdictOK)
    );
    assert_eq!(result.result[1].status, RunStatus::WrongAnswer);
    assert_eq!(result.result[2].status, RunStatus::Skipped);

    // all-or-nothing: testcase 1's initial score gets zeroed too, since the
    // subtask as a whole didn't pass.
    assert!(result.result.iter().all(|r| r.score == 0));
    assert_eq!(result.score, 0);
}

#[test]
fn subtasks_no_skip_runs_rest() {
    let root = TempRoot::new("noskip");
    let task = with_subtasks(false, &[(100, 3)], &root);
    let mut sandbox = FakeSandbox::with_verdicts([
        RunVerdict::VerdictOK,
        RunVerdict::VerdictOK,
        RunVerdict::VerdictOK,
    ])
    .checks([true, false, true]);

    let result = judge(&task, &CPP, &mut sandbox).unwrap();

    // all 3 actually ran (no Skipped) - proves skip=false disables the
    // short-circuit rather than just changing the Skipped label.
    assert!(result.result.iter().all(|r| r.status != RunStatus::Skipped));
    assert_eq!(result.result[1].status, RunStatus::WrongAnswer);
    // still all-or-nothing scoring: one failure zeroes the whole subtask.
    assert_eq!(result.score, 0);
}

#[test]
fn subtasks_independent_scoring() {
    let root = TempRoot::new("independent");
    let task = with_subtasks(true, &[(30, 1), (70, 2)], &root);
    let mut sandbox = FakeSandbox::with_verdicts([
        RunVerdict::VerdictOK, // subtask 1, testcase 1: fails
        RunVerdict::VerdictOK, // subtask 2, testcase 1: passes
        RunVerdict::VerdictOK, // subtask 2, testcase 2: passes
    ])
    .checks([false, true, true]);

    let result = judge(&task, &CPP, &mut sandbox).unwrap();

    assert_eq!(result.score, 70);
    assert_eq!(result.result[0].subtask_index, 1);
    assert_eq!(result.result[1].subtask_index, 2);
    assert_eq!(result.result[2].subtask_index, 2);
}

#[test]
fn compile_failure_is_compilation_error() {
    let root = TempRoot::new("ce");
    let task = normal(100, 2, &root);
    let mut sandbox = FakeSandbox::with_verdicts([]);
    sandbox.compiles = false;

    let result = judge(&task, &CPP, &mut sandbox).unwrap();

    assert_eq!(result.status, JudgeStatus::CompilationError);
    assert!(result.result.is_empty());
    assert_eq!(sandbox.compile_calls, 1);
}

#[test]
fn missing_testcases_skips_compile() {
    let root = TempRoot::new("missing");
    let task = root.task(
        r#""skip": false, "full_score": 100, "num_testcases": 2, "subtasks": []"#,
        1,
    );
    let mut sandbox = FakeSandbox::with_verdicts([]);

    let result = judge(&task, &CPP, &mut sandbox).unwrap();

    assert_eq!(result.status, JudgeStatus::TestcasesError);
    assert_eq!(sandbox.compile_calls, 0);
}

#[test]
fn disallowed_language_is_rejected() {
    let root = TempRoot::new("lang");
    let task = root.task(
        r#""skip": false, "full_score": 100, "num_testcases": 1, "subtasks": [], "allowed_languages": ["python"]"#,
        1,
    );
    let mut sandbox = FakeSandbox::with_verdicts([]);

    let err = judge(&task, &CPP, &mut sandbox).unwrap_err().to_string();

    assert!(err.contains("Language 'cpp' is not allowed for task 't'"));
    assert!(err.contains("python"));
    assert_eq!(sandbox.compile_calls, 0);
}
