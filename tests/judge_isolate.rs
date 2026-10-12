use judge_ma_di::judge::isolate::{parse_meta, IsolateResult, RunVerdict};

const LIMIT_KB: u64 = 262_144;

fn parse(meta: &str) -> IsolateResult {
    parse_meta(meta, LIMIT_KB).unwrap()
}

#[test]
fn parse_meta_ok_without_status() {
    let res = parse("time:0.042\ntime-wall:0.050\ncg-mem:1024\nexitcode:0\n");
    assert_eq!(
        res,
        IsolateResult {
            status: RunVerdict::VerdictOK,
            time_usage: 0.042,
            memory_usage: 1024,
        }
    );
}

#[test]
fn parse_meta_maps_status_codes() {
    for (code, verdict) in [
        ("TO", RunVerdict::VerdictTLE),
        ("RE", RunVerdict::VerdictRE),
        ("SG", RunVerdict::VerdictSG),
        ("XX", RunVerdict::VerdictXX),
    ] {
        let res = parse(&format!("status:{code}\ncg-mem:1024\n"));
        assert_eq!(res.status, verdict);
    }
}

#[test]
fn parse_meta_oom_killed_is_mle() {
    let res = parse("status:SG\ncg-mem:1024\ncg-oom-killed:1\n");
    assert_eq!(res.status, RunVerdict::VerdictMLE);
}

#[test]
fn parse_meta_memory_at_limit_is_mle() {
    let res = parse(&format!("cg-mem:{LIMIT_KB}\n"));
    assert_eq!(res.status, RunVerdict::VerdictMLE);
}

#[test]
fn parse_meta_rejects_malformed_numbers() {
    assert!(parse_meta("time:abc\n", LIMIT_KB).is_err());
}
