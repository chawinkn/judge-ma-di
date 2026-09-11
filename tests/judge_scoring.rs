use judge_ma_di::judge::config::Subtask;
use judge_ma_di::judge::isolate::{IsolateResult, RunVerdict, Sandbox};
use judge_ma_di::judge::runner::{run_each, run_normal, run_subtask, RunStatus};
use std::collections::VecDeque;

// Scripted sandbox: run()/check() results are consumed in order, and panic
// if called more times than scripted - that's how we prove skip-on-failure
// (and the "no check() when the verdict isn't OK" branch) actually stop
// calling the sandbox instead of just producing a zero score some other way.
struct FakeSandbox {
    run_results: VecDeque<IsolateResult>,
    check_results: VecDeque<bool>,
}

impl FakeSandbox {
    fn with_verdicts(verdicts: impl IntoIterator<Item = RunVerdict>) -> Self {
        Self {
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
fn run_each_wrong_answer() {
    let mut sandbox = FakeSandbox::with_verdicts([RunVerdict::VerdictOK]).checks([false]);

    let result = run_each(&mut sandbox, 50, 0, 1).unwrap();

    assert_eq!(result.status, RunStatus::WrongAnswer);
    assert_eq!(result.score, 0);
}

#[test]
fn run_each_skips_checker_on_bad_verdict() {
    // No entries in check_results - a check() call here would panic, so this
    // also proves the checker is skipped entirely for a non-OK verdict.
    let mut sandbox = FakeSandbox::with_verdicts([RunVerdict::VerdictTLE]);

    let result = run_each(&mut sandbox, 50, 0, 1).unwrap();

    assert_eq!(result.status, RunStatus::Verdict(RunVerdict::VerdictTLE));
    assert_eq!(result.score, 0);
}

#[test]
fn run_normal_partial_score() {
    // Unlike subtasks, flat scoring isn't all-or-nothing: testcase 1 and 4
    // keep their score even though 2 and 3 fail.
    let mut sandbox = FakeSandbox::with_verdicts([
        RunVerdict::VerdictOK,
        RunVerdict::VerdictOK,
        RunVerdict::VerdictTLE,
        RunVerdict::VerdictOK,
    ])
    .checks([true, false, true]);

    let result = run_normal(&mut sandbox, 100, 4).unwrap();

    assert_eq!(result.result.len(), 4);
    assert_eq!(result.result[0].score, 25);
    assert_eq!(result.result[1].score, 0);
    assert_eq!(result.result[2].score, 0);
    assert_eq!(result.result[3].score, 25);
    assert_eq!(result.score, 50);
}

#[test]
fn run_normal_time_memory_max() {
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
    };

    let result = run_normal(&mut sandbox, 100, 2).unwrap();

    assert_eq!(result.time, 500); // 0.5s -> 500ms, the slower of the two
    assert_eq!(result.memory, 1000); // the larger of the two
}

#[test]
fn subtasks_skip_zeroes_rest() {
    let mut sandbox = FakeSandbox::with_verdicts([RunVerdict::VerdictOK, RunVerdict::VerdictOK])
        .checks([true, false]);
    let subtasks = vec![Subtask {
        full_score: 100,
        num_testcases: 3,
    }];

    let result = run_subtask(&mut sandbox, subtasks, true).unwrap();

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
    let mut sandbox = FakeSandbox::with_verdicts([
        RunVerdict::VerdictOK,
        RunVerdict::VerdictOK,
        RunVerdict::VerdictOK,
    ])
    .checks([true, false, true]);
    let subtasks = vec![Subtask {
        full_score: 100,
        num_testcases: 3,
    }];

    let result = run_subtask(&mut sandbox, subtasks, false).unwrap();

    // all 3 actually ran (no Skipped) - proves use_skip=false disables the
    // short-circuit rather than just changing the Skipped label.
    assert!(result.result.iter().all(|r| r.status != RunStatus::Skipped));
    assert_eq!(result.result[1].status, RunStatus::WrongAnswer);
    // still all-or-nothing scoring: one failure zeroes the whole subtask.
    assert_eq!(result.score, 0);
}

#[test]
fn subtasks_independent_scoring() {
    let mut sandbox = FakeSandbox::with_verdicts([
        RunVerdict::VerdictOK, // subtask 1, testcase 1: fails
        RunVerdict::VerdictOK, // subtask 2, testcase 1: passes
        RunVerdict::VerdictOK, // subtask 2, testcase 2: passes
    ])
    .checks([false, true, true]);
    let subtasks = vec![
        Subtask {
            full_score: 30,
            num_testcases: 1,
        },
        Subtask {
            full_score: 70,
            num_testcases: 2,
        },
    ];

    let result = run_subtask(&mut sandbox, subtasks, true).unwrap();

    assert_eq!(result.score, 70);
    assert_eq!(result.result[0].subtask_index, 1);
    assert_eq!(result.result[1].subtask_index, 2);
    assert_eq!(result.result[2].subtask_index, 2);
}
