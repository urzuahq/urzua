//! MILE-129: opt-in evidence pins, a drift grace window and a claim-age
//! warning. Real git-repo fixtures, real binary; all output is JSON. Every
//! option is off by default, so each test that turns one on has a sibling
//! asserting the default behaviour is unchanged.

mod common;

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str], date: Option<&str>) {
    let mut cmd = Command::new("git");
    cmd.args(args).current_dir(dir);
    if let Some(d) = date {
        cmd.env("GIT_AUTHOR_DATE", d).env("GIT_COMMITTER_DATE", d);
    }
    assert!(
        cmd.status().expect("git should run").success(),
        "git {args:?} failed"
    );
}

fn commit_all(dir: &Path, date: Option<&str>) {
    git(dir, &["add", "-A"], None);
    git(dir, &["commit", "-q", "-m", "fixture"], date);
}

/// Every run is evaluated as of the date the fixture was built on, passed explicitly, so the
/// fixture's `Audited` date and the process's evaluation date cannot straddle midnight UTC.
fn run_urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    common::urzua()
        .args(["--as-of", &today()])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

fn json(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

const RECORD_PATH: &str = "docs/adr/ADR-1-first.md";
const OLD: &str = "2020-01-01T00:00:00";

fn config(consistency: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n  embodiment.consistency: {consistency}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields: [\"Embodiment\", \"Realized-by\"]\n"
    )
}

fn record(realized_by: &str) -> String {
    record_stating("Implemented", realized_by)
}

fn record_stating(state: &str, realized_by: &str) -> String {
    format!(
        "---\nEmbodiment: '{state}'\nRealized-by: '{realized_by}'\n---\n# ADR-1: First\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
    )
}

fn sha256(content: &str) -> String {
    let path = common::TempFile::new("urzua-pin");
    std::fs::write(&path, content).unwrap();
    let out = Command::new("sha256sum").arg(&path).output().unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_string()
}

fn repo(name: &str, consistency: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-pins-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"], None);
    git(&dir, &["config", "user.email", "test@example.com"], None);
    git(&dir, &["config", "user.name", "test"], None);
    std::fs::write(dir.join(".urzua/config.yaml"), config(consistency)).unwrap();
    dir
}

fn consistency_messages(parsed: &serde_json::Value) -> Vec<String> {
    parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "embodiment.consistency")
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect()
}

const V1: &str = "fn a() {}\n";
const V2: &str = "fn a() { let _ = 1; }\n";

/// The locator changes and changes back after the record was written: git
/// history alone calls this drift, a content pin sees identical bytes.
#[test]
fn a_pin_that_still_matches_overrides_the_git_history_heuristic_observed_failing() {
    let dir = repo("pin-holds", "error");
    std::fs::write(dir.join("src/a.rs"), V1).unwrap();
    std::fs::write(
        dir.join(RECORD_PATH),
        record(&format!("code:src/a.rs@sha256:{}", sha256(V1))),
    )
    .unwrap();
    commit_all(&dir, None);
    std::fs::write(dir.join("src/a.rs"), V2).unwrap();
    commit_all(&dir, None);
    std::fs::write(dir.join("src/a.rs"), V1).unwrap();
    commit_all(&dir, None);

    let parsed = json(&run_urzua(&dir, &["check", "docs/"]));
    assert_eq!(
        consistency_messages(&parsed),
        Vec::<String>::new(),
        "{parsed}"
    );

    // The same history without a pin is the heuristic's drift.
    let unpinned = std::fs::read_to_string(dir.join(RECORD_PATH))
        .unwrap()
        .replace(&format!("@sha256:{}", sha256(V1)), "");
    std::fs::write(dir.join(RECORD_PATH), unpinned).unwrap();
    git(&dir, &["add", "-A"], None);
    git(&dir, &["commit", "-q", "-m", "unpin"], None);
    std::fs::write(dir.join("src/a.rs"), V2).unwrap();
    commit_all(&dir, None);
    let parsed = json(&run_urzua(&dir, &["check", "docs/"]));
    assert!(
        consistency_messages(&parsed)
            .iter()
            .any(|m| m.contains("Drift detected")),
        "{parsed}"
    );
}

/// The record was written after the locator's last commit, so the heuristic
/// sees no drift; the pinned hash is of older content, so the pin does.
#[test]
fn a_pin_that_no_longer_matches_is_drift_even_when_the_record_is_newer_observed_failing() {
    let dir = repo("pin-differs", "error");
    std::fs::write(dir.join("src/a.rs"), V1).unwrap();
    commit_all(&dir, None);
    std::fs::write(dir.join("src/a.rs"), V2).unwrap();
    commit_all(&dir, None);
    std::fs::write(
        dir.join(RECORD_PATH),
        record(&format!("code:src/a.rs@sha256:{}", sha256(V1))),
    )
    .unwrap();
    commit_all(&dir, None);

    let parsed = json(&run_urzua(&dir, &["check", "docs/"]));
    assert!(
        consistency_messages(&parsed)
            .iter()
            .any(|m| m.contains("Drift detected")),
        "{parsed}"
    );
    // The locator still resolves: a pin suffix is not part of the path.
    let all = parsed["findings"].as_array().unwrap();
    assert!(
        all.iter().all(|f| f["rule"] != "embodiment.locator-exists"),
        "{parsed}"
    );
}

fn drifted_repo(name: &str, consistency: &str, locator_date: Option<&str>) -> common::TempDir {
    let dir = repo(name, consistency);
    std::fs::write(dir.join("src/a.rs"), V1).unwrap();
    std::fs::write(dir.join(RECORD_PATH), record("code:src/a.rs")).unwrap();
    commit_all(&dir, Some(OLD));
    std::fs::write(dir.join("src/a.rs"), V2).unwrap();
    commit_all(&dir, locator_date);
    dir
}

#[test]
fn a_fresh_locator_change_inside_the_grace_window_does_not_report_observed_failing() {
    let dir = drifted_repo("grace-fresh", "{level: error, drift_grace_days: 7}", None);
    let parsed = json(&run_urzua(&dir, &["check", "docs/"]));
    assert_eq!(
        consistency_messages(&parsed),
        Vec::<String>::new(),
        "{parsed}"
    );
}

#[test]
fn a_locator_change_older_than_the_grace_window_reports() {
    let dir = drifted_repo(
        "grace-old",
        "{level: error, drift_grace_days: 7}",
        Some("2021-01-01T00:00:00"),
    );
    let parsed = json(&run_urzua(&dir, &["check", "docs/"]));
    assert!(
        consistency_messages(&parsed)
            .iter()
            .any(|m| m.contains("Drift detected")),
        "{parsed}"
    );
}

#[test]
fn with_no_grace_window_a_fresh_change_reports_as_before() {
    let dir = drifted_repo("grace-off", "error", None);
    let parsed = json(&run_urzua(&dir, &["check", "docs/"]));
    assert!(
        consistency_messages(&parsed)
            .iter()
            .any(|m| m.contains("Drift detected")),
        "{parsed}"
    );
}

fn aged_repo(name: &str, consistency: &str) -> common::TempDir {
    let dir = repo(name, consistency);
    std::fs::write(dir.join("src/a.rs"), V1).unwrap();
    std::fs::write(dir.join(RECORD_PATH), record("code:src/a.rs")).unwrap();
    commit_all(&dir, Some(OLD));
    dir
}

#[test]
fn a_claim_older_than_the_configured_age_warns_observed_failing() {
    let dir = aged_repo("age-old", "{level: warn, claim_max_age_days: 365}");
    let parsed = json(&run_urzua(&dir, &["check", "docs/"]));
    let messages = consistency_messages(&parsed);
    assert_eq!(messages.len(), 1, "{parsed}");
    assert!(
        messages[0].contains("last verified") && messages[0].contains("365"),
        "{messages:?}"
    );
    let finding = parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["rule"] == "embodiment.consistency")
        .unwrap();
    assert_eq!(finding["severity"], "warning", "{parsed}");
}

#[test]
fn a_claim_younger_than_the_configured_age_is_silent() {
    let dir = repo("age-young", "{level: error, claim_max_age_days: 365}");
    std::fs::write(dir.join("src/a.rs"), V1).unwrap();
    std::fs::write(dir.join(RECORD_PATH), record("code:src/a.rs")).unwrap();
    commit_all(&dir, None);
    let parsed = json(&run_urzua(&dir, &["check", "docs/"]));
    assert_eq!(
        consistency_messages(&parsed),
        Vec::<String>::new(),
        "{parsed}"
    );
}

#[test]
fn without_a_claim_age_an_old_claim_is_silent() {
    let dir = aged_repo("age-off", "error");
    let parsed = json(&run_urzua(&dir, &["check", "docs/"]));
    assert_eq!(
        consistency_messages(&parsed),
        Vec::<String>::new(),
        "{parsed}"
    );
}

#[test]
fn the_new_options_belong_to_embodiment_consistency_only() {
    let dir = repo("option-scope", "error");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        config("error").replace(
            "embodiment.consistency: error",
            "embodiment.consistency: error\n  header.required-fields: {level: error, drift_grace_days: 3}",
        ),
    )
    .unwrap();
    let out = run_urzua(&dir, &["check", "docs/"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("drift_grace_days"),
        "{out:?}"
    );
}

/// The record is committed first and the locator changes after, so the drift
/// is fresh. The record already says `Drift detected`.
fn stated_drift_repo(name: &str, locator_date: Option<&str>) -> common::TempDir {
    let dir = repo(name, "{level: error, drift_grace_days: 7}");
    std::fs::write(dir.join("src/a.rs"), V1).unwrap();
    std::fs::write(
        dir.join(RECORD_PATH),
        record_stating("Drift detected", "code:src/a.rs"),
    )
    .unwrap();
    commit_all(&dir, Some(OLD));
    std::fs::write(dir.join("src/a.rs"), V2).unwrap();
    commit_all(&dir, locator_date);
    dir
}

#[test]
fn a_stated_drift_detected_is_not_contradicted_by_fresh_drift_inside_the_grace_window_observed_failing(
) {
    let dir = stated_drift_repo("grace-stated", None);
    let parsed = json(&run_urzua(&dir, &["check", "docs/"]));
    assert_eq!(
        consistency_messages(&parsed),
        Vec::<String>::new(),
        "{parsed}"
    );
}

#[test]
fn a_stated_tier_is_still_accepted_for_fresh_drift_inside_the_grace_window() {
    let dir = drifted_repo("grace-tier", "{level: error, drift_grace_days: 7}", None);
    let parsed = json(&run_urzua(&dir, &["check", "docs/"]));
    assert_eq!(
        consistency_messages(&parsed),
        Vec::<String>::new(),
        "{parsed}"
    );
}

#[test]
fn a_stated_drift_detected_agrees_with_drift_older_than_the_grace_window() {
    let dir = stated_drift_repo("grace-stated-old", Some("2021-01-01T00:00:00"));
    let parsed = json(&run_urzua(&dir, &["check", "docs/"]));
    assert_eq!(
        consistency_messages(&parsed),
        Vec::<String>::new(),
        "{parsed}"
    );
}

#[test]
fn fix_does_not_flip_a_stated_drift_detected_while_the_drift_is_inside_the_grace_window_observed_failing(
) {
    let dir = stated_drift_repo("grace-fix", None);
    let parsed = json(&run_urzua(&dir, &["fix"]));
    assert_eq!(
        parsed["repairs"].as_array().map(Vec::len),
        Some(0),
        "{parsed}"
    );
}

/// `MILE-166`: a declared last-audit field replaces the locator's last change
/// as where the grace window counts from.
fn audit_repo(name: &str, locator_date: Option<&str>, audited: Option<&str>) -> common::TempDir {
    let dir = repo(
        name,
        "{level: error, drift_grace_days: 7, drift_audit_field: Audited}",
    );
    let cfg = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        cfg.replace("\"Realized-by\"]", "\"Realized-by\", \"Audited\"]"),
    )
    .unwrap();
    std::fs::write(dir.join("src/a.rs"), V1).unwrap();
    let mut text = record("code:src/a.rs");
    if let Some(a) = audited {
        text = text.replacen("---\n#", &format!("Audited: '{a}'\n---\n#"), 1);
    }
    std::fs::write(dir.join(RECORD_PATH), text).unwrap();
    commit_all(&dir, Some(OLD));
    std::fs::write(dir.join("src/a.rs"), V2).unwrap();
    commit_all(&dir, locator_date);
    dir
}

/// The date this test process's fixtures are built on: read once, so every use agrees.
fn today() -> String {
    static TODAY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    TODAY
        .get_or_init(|| {
            let out = Command::new("date").args(["-u", "+%F"]).output().unwrap();
            String::from_utf8(out.stdout).unwrap().trim().to_string()
        })
        .clone()
}

fn drift_reported(parsed: &serde_json::Value) -> bool {
    consistency_messages(parsed)
        .iter()
        .any(|m| m.contains("Drift detected"))
}

#[test]
fn a_recent_last_audit_holds_back_drift_the_locator_dates_would_report() {
    // The locator changed long ago, so the file's last change is past the
    // window; the record was audited today, so the window is not.
    let dir = audit_repo("audit-fresh", Some("2021-01-01T00:00:00"), Some(&today()));
    let parsed = json(&run_urzua(&dir, &["check", "docs/"]));
    assert!(!drift_reported(&parsed), "{parsed}");
}

#[test]
fn an_old_last_audit_reports_drift_the_locator_dates_would_hold_back() {
    // The locator changed just now (inside the window by file date), but the
    // declared audit is old, and it wins.
    let dir = audit_repo("audit-old", None, Some("2021-01-01"));
    let parsed = json(&run_urzua(&dir, &["check", "docs/"]));
    assert!(drift_reported(&parsed), "{parsed}");
}

#[test]
fn a_record_without_the_audit_field_counts_from_the_files_last_change() {
    let dir = audit_repo("audit-absent-fresh", None, None);
    let parsed = json(&run_urzua(&dir, &["check", "docs/"]));
    assert!(!drift_reported(&parsed), "{parsed}");
    let dir = audit_repo("audit-absent-old", Some("2021-01-01T00:00:00"), None);
    let parsed = json(&run_urzua(&dir, &["check", "docs/"]));
    assert!(drift_reported(&parsed), "{parsed}");
}

#[test]
fn a_malformed_last_audit_is_reported_and_falls_back_to_the_files_last_change() {
    let dir = audit_repo("audit-bad", None, Some("2026-13-45"));
    let parsed = json(&run_urzua(&dir, &["check", "docs/"]));
    let messages = consistency_messages(&parsed);
    assert!(
        messages
            .iter()
            .any(|m| m.contains("last-audit field Audited")),
        "{parsed}"
    );
    // The fresh locator change is still held back by the file's date.
    assert!(!drift_reported(&parsed), "{parsed}");
}
