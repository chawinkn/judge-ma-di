use judge_ma_di::judge::runner::{run, JudgeStatus};
use std::path::Path;

fn require_isolate_cgroup() {
    assert!(
        Path::new("/run/isolate/cgroup").exists(),
        "/run/isolate/cgroup not found! Start isolate-cg-keeper and run 'sudo isolate --cg --init' before running isolate integration tests."
    );
}

#[test]
fn test_isolate_cpp_scores_100() {
    require_isolate_cgroup();
    let code = "#include <iostream>\nint main() { long long a, b; if (std::cin >> a >> b) std::cout << a + b << std::endl; }";
    let res = run("a_plus_b".into(), 81, code.into(), "cpp".into()).unwrap();

    assert_eq!(res.status, JudgeStatus::Completed);
    assert_eq!(res.score, 100);
    assert_eq!(res.result.len(), 10);
}

#[test]
fn test_isolate_python_scores_100() {
    require_isolate_cgroup();
    let code = "import sys\nlines = sys.stdin.read().split()\nif lines: print(int(lines[0]) + int(lines[1]))";
    let res = run("a_plus_b".into(), 82, code.into(), "python".into()).unwrap();

    assert_eq!(res.status, JudgeStatus::Completed);
    assert_eq!(res.score, 100);
    assert_eq!(res.result.len(), 10);
}

#[test]
fn test_isolate_exploit_cannot_read_solutions() {
    require_isolate_cgroup();
    let code = "#include <iostream>\n#include <fstream>\nint main() { std::ifstream s(\"/testcases/1.sol\"); if (s.is_open()) std::cout << s.rdbuf(); else std::cout << \"ACCESS_DENIED\"; }";
    let res = run("a_plus_b".into(), 83, code.into(), "cpp".into()).unwrap();

    assert_eq!(res.status, JudgeStatus::Completed);
    assert_eq!(res.score, 0);
}

#[test]
fn test_isolate_compilation_error_handled() {
    require_isolate_cgroup();
    let code = "int main() { syntax error here }";
    let res = run("a_plus_b".into(), 84, code.into(), "cpp".into()).unwrap();

    assert_eq!(res.status, JudgeStatus::CompilationError);
    assert_eq!(res.score, 0);
    assert!(res.result.is_empty());
}
