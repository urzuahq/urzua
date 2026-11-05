//! MILE-154: a pair's provisional split, run through this repository's own
//! `.urzua/config.yaml` (the `amendment` pair) against planted RFCs.

mod common;

use std::path::Path;
use std::process::Command;

fn run_with_own_config(name: &str, files: &[(&str, &str)]) -> serde_json::Value {
    let dir = common::TempDir::new(&format!("urzua-provisional-{name}"));
    std::fs::create_dir_all(dir.join("docs/rfc")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    let config = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.urzua/config.yaml");
    std::fs::copy(config, dir.join(".urzua/config.yaml")).unwrap();
    for (file, body) in files {
        std::fs::write(dir.join("docs/rfc").join(file), body).unwrap();
    }
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

fn rfc(status: &str, extra: &str) -> String {
    format!("---\nStatus: {status}\nDate: 2026-11-04\nAuthor: Ana\n{extra}---\n# 1 - T\n")
}

fn pair_severities(report: &serde_json::Value) -> Vec<String> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "relation.pair-reciprocity")
        .map(|f| f["severity"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_draft_amendment_the_peer_has_not_echoed_is_a_warning() {
    let report = run_with_own_config(
        "draft",
        &[
            ("RFC-1-base.md", &rfc("Accepted", "")),
            ("RFC-2-amender.md", &rfc("Draft", "Amends: RFC-1\n")),
        ],
    );
    assert_eq!(pair_severities(&report), vec!["warning"], "{report}");
}

#[test]
fn an_accepted_amendment_the_peer_has_not_echoed_is_an_error() {
    let report = run_with_own_config(
        "accepted",
        &[
            ("RFC-1-base.md", &rfc("Accepted", "")),
            ("RFC-2-amender.md", &rfc("Accepted", "Amends: RFC-1\n")),
        ],
    );
    assert_eq!(pair_severities(&report), vec!["error"], "{report}");
}

#[test]
fn an_echoed_draft_amendment_is_not_reported() {
    let report = run_with_own_config(
        "echoed",
        &[
            ("RFC-1-base.md", &rfc("Accepted", "Amended-by: RFC-2\n")),
            ("RFC-2-amender.md", &rfc("Draft", "Amends: RFC-1\n")),
        ],
    );
    assert!(pair_severities(&report).is_empty(), "{report}");
}

/// MILE-163: the peer's declared acknowledgement field settles a provisional
/// claim, and only a provisional one.
#[test]
fn a_draft_amendment_the_peer_acknowledges_is_not_reported_observed_failing() {
    let report = run_with_own_config(
        "acknowledged",
        &[
            (
                "RFC-1-base.md",
                &rfc("Accepted", "Amended-by-pending: RFC-2\n"),
            ),
            ("RFC-2-amender.md", &rfc("Draft", "Amends: RFC-1\n")),
        ],
    );
    assert!(pair_severities(&report).is_empty(), "{report}");
}

#[test]
fn an_accepted_amendment_is_not_settled_by_an_acknowledgement_observed_failing() {
    let report = run_with_own_config(
        "accepted-acknowledged",
        &[
            (
                "RFC-1-base.md",
                &rfc("Accepted", "Amended-by-pending: RFC-2\n"),
            ),
            ("RFC-2-amender.md", &rfc("Accepted", "Amends: RFC-1\n")),
        ],
    );
    assert_eq!(pair_severities(&report), vec!["error"], "{report}");
}
