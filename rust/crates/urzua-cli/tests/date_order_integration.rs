//! MILE-157: cross-field date ordering, run through this repository's own
//! `.urzua/config.yaml` against planted ADRs.

use std::path::Path;
use std::process::Command;

fn run(name: &str, extra: &str) -> serde_json::Value {
    let dir = std::env::temp_dir().join(format!("urzua-date-order-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    let config = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.urzua/config.yaml");
    std::fs::copy(config, dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-first.md"),
        format!("---\nStatus: Accepted\nDate: 2026-03-02\nAuthor: Ana\nDeciders: Ana\n{extra}---\n# 1 - First\n"),
    )
    .unwrap();
    for args in [&["init", "-q"][..], &["add", "-A"][..]] {
        assert!(Command::new("git")
            .args(args)
            .current_dir(&dir)
            .status()
            .unwrap()
            .success());
    }
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(["check", "docs/"])
        .current_dir(&dir)
        .output()
        .expect("urzua binary should run");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {out:?}"))
}

fn order_findings(report: &serde_json::Value) -> usize {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "date.field-order")
        .count()
}

#[test]
fn an_adr_accepted_before_it_was_written_is_reported() {
    let report = run("before", "Accepted-on: 2026-03-01\n");
    assert_eq!(order_findings(&report), 1, "{report}");
}

#[test]
fn equal_later_and_absent_acceptance_dates_pass() {
    for (n, extra) in [
        ("equal", "Accepted-on: 2026-03-02\n"),
        ("later", "Accepted-on: 2026-04-01\n"),
        ("absent", ""),
        ("malformed", "Accepted-on: soon\n"),
    ] {
        let report = run(n, extra);
        assert_eq!(order_findings(&report), 0, "{n}: {report}");
    }
}
