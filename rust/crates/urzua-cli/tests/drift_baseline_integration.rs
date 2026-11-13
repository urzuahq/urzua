//! MILE-49: `embodiment.consistency`'s `drift_baseline` holds drift that predates a
//! git ref out of the gate and keeps reporting drift introduced since. Real git
//! fixtures, real binary; all output is JSON. The option is opt-in, so the first
//! test pins the default.

mod common;

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git should run");
    assert!(out.status.success(), "git {args:?} failed: {out:?}");
}

fn commit_all(dir: &Path, message: &str) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", message]);
}

fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

fn json(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

fn config(option: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n  embodiment.consistency:\n    level: error\n{option}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields: [\"Embodiment\", \"Realized-by\"]\n"
    )
}

fn record(n: u8, state: &str, file: &str) -> String {
    format!(
        "---\nEmbodiment: '{state}'\nRealized-by: 'code:src/{file}'\n---\n# ADR-{n}: Record {n}\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
    )
}

/// ADR-1 cites `src/a.rs` and ADR-2 cites `src/b.rs`; `a.rs` changes after both
/// records, then the baseline is tagged, then `b.rs` changes: ADR-1's drift
/// predates the baseline and ADR-2's does not.
fn repo(name: &str, option: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-baseline-{name}"));
    for d in ["docs/adr", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config(option)).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join("src/b.rs"), "fn b() {}\n").unwrap();
    std::fs::write(dir.join("src/c.rs"), "fn c() {}\n").unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-record-1.md"),
        record(1, "Implemented", "a.rs"),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-2-record-2.md"),
        record(2, "Implemented", "b.rs"),
    )
    .unwrap();
    commit_all(&dir, "records");
    std::fs::write(dir.join("src/a.rs"), "fn a() { let _ = 1; }\n").unwrap();
    commit_all(&dir, "a changes");
    git(&dir, &["tag", "adoption"]);
    std::fs::write(dir.join("src/b.rs"), "fn b() { let _ = 1; }\n").unwrap();
    commit_all(&dir, "b changes");
    dir
}

/// The record after someone re-checked it: its `Realized-by` line is rewritten (the
/// line drift is dated from), here by citing one more file that has not changed.
fn rechecked(n: u8, file: &str) -> String {
    record(n, "Implemented", file).replace(
        &format!("'code:src/{file}'"),
        &format!("'code:src/{file}, code:src/c.rs'"),
    )
}

const BASELINE: &str = "    drift_baseline: {ref: adoption}";

fn drift_findings(parsed: &serde_json::Value) -> Vec<String> {
    let mut paths: Vec<String> = parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "embodiment.consistency")
        .map(|f| f["file"].as_str().unwrap().to_string())
        .collect();
    paths.sort();
    paths
}

fn notice<'a>(parsed: &'a serde_json::Value, subject: &str) -> Option<&'a serde_json::Value> {
    parsed["notices"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|n| n["subject"] == subject)
}

#[test]
fn without_drift_baseline_every_drifted_record_is_reported_and_no_baseline_is_stated() {
    let dir = repo("default", "");
    let out = run(&dir, &["check"]);
    let parsed = json(&out);
    assert_eq!(
        drift_findings(&parsed),
        ["docs/adr/ADR-1-record-1.md", "docs/adr/ADR-2-record-2.md"],
        "{parsed}"
    );
    assert!(parsed.get("baseline").is_none(), "{parsed}");
    assert!(notice(&parsed, "embodiment-drift-baselined").is_none());
}

#[test]
fn a_baselined_record_and_a_newly_drifted_one_are_each_reported_correctly() {
    let dir = repo("split", BASELINE);
    let out = run(&dir, &["check"]);
    let parsed = json(&out);
    // Only the drift introduced since the baseline is a finding, and only it blocks.
    assert_eq!(
        drift_findings(&parsed),
        ["docs/adr/ADR-2-record-2.md"],
        "{parsed}"
    );
    assert_eq!(out.status.code(), Some(1), "{parsed}");
    let baseline = &parsed["baseline"];
    assert_eq!(baseline["ref"], "adoption", "{parsed}");
    assert_eq!(baseline["pre_existing"], 1);
    assert_eq!(baseline["introduced"], 1);
    assert_eq!(
        baseline["pre_existing_records"],
        serde_json::json!(["docs/adr/ADR-1-record-1.md"])
    );
    assert_eq!(baseline["commit"].as_str().unwrap().len(), 40);
    let held =
        notice(&parsed, "embodiment-drift-baselined").expect("a notice names the held-out drift");
    assert_eq!(held["severity"], "info");
    assert_eq!(held["detail"]["count"], "1");
}

#[test]
fn pre_existing_drift_alone_does_not_block_even_under_fail_on_warning() {
    let dir = repo("only-old", BASELINE);
    // ADR-2 is brought back in line: its record is touched after `b.rs` changed.
    std::fs::write(dir.join("docs/adr/ADR-2-record-2.md"), rechecked(2, "b.rs")).unwrap();
    commit_all(&dir, "re-verify ADR-2");
    let out = run(&dir, &["check", "--fail-on", "warning"]);
    let parsed = json(&out);
    assert_eq!(drift_findings(&parsed), Vec::<String>::new(), "{parsed}");
    assert_eq!(out.status.code(), Some(0), "{parsed}");
    assert_eq!(parsed["baseline"]["pre_existing"], 1);
    assert_eq!(parsed["baseline"]["introduced"], 0);
}

#[test]
fn a_baselined_record_may_state_the_drift_state_and_still_agrees() {
    let dir = repo("stated", BASELINE);
    // `urzua fix` would have written the drift state; a baselined record agrees with it too.
    let stated = record(1, "Drift detected", "a.rs");
    std::fs::write(dir.join("docs/adr/ADR-1-record-1.md"), stated).unwrap();
    commit_all(&dir, "state the drift");
    let parsed = json(&run(&dir, &["check"]));
    assert!(
        !drift_findings(&parsed).contains(&"docs/adr/ADR-1-record-1.md".to_string()),
        "{parsed}"
    );
}

#[test]
fn drift_a_cited_file_gained_after_the_baseline_is_introduced_drift() {
    let dir = repo("moved", BASELINE);
    std::fs::write(dir.join("src/a.rs"), "fn a() { let _ = 2; }\n").unwrap();
    commit_all(&dir, "a changes again");
    let parsed = json(&run(&dir, &["check"]));
    assert_eq!(
        drift_findings(&parsed),
        ["docs/adr/ADR-1-record-1.md", "docs/adr/ADR-2-record-2.md"],
        "{parsed}"
    );
    assert_eq!(parsed["baseline"]["pre_existing"], 0);
}

#[test]
fn a_baseline_ref_that_names_no_commit_stops_the_run() {
    let dir = repo("bad-ref", "    drift_baseline: {ref: no-such-ref}");
    let out = run(&dir, &["check"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let text =
        String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    assert!(
        text.contains("drift_baseline") && text.contains("no-such-ref"),
        "{text}"
    );
}

#[test]
fn a_blank_or_option_like_ref_is_a_config_error() {
    for (name, bad) in [("blank", "\"\""), ("dash", "--all")] {
        let dir = repo(name, &format!("    drift_baseline: {{ref: {bad}}}"));
        let out = run(&dir, &["check"]);
        assert_eq!(out.status.code(), Some(2), "{bad}: {out:?}");
        let text = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        assert!(text.contains("drift_baseline"), "{bad}: {text}");
    }
}

#[test]
fn a_baseline_that_holds_out_nothing_is_nudged_for_removal_on_a_whole_corpus_run_only() {
    let dir = repo("unused", BASELINE);
    // Re-verify ADR-1 after its drift: nothing predates the baseline any more.
    std::fs::write(dir.join("docs/adr/ADR-1-record-1.md"), rechecked(1, "a.rs")).unwrap();
    commit_all(&dir, "re-verify ADR-1");
    let parsed = json(&run(&dir, &["check"]));
    let nudge = notice(&parsed, "embodiment-drift-baseline-unused").expect("a nudge");
    assert_eq!(nudge["severity"], "info");
    assert_eq!(parsed["baseline"]["pre_existing"], 0);

    let scoped = json(&run(&dir, &["check", "docs/adr/ADR-1-record-1.md"]));
    assert!(
        notice(&scoped, "embodiment-drift-baseline-unused").is_none(),
        "a scoped run proves nothing about the rest of the corpus: {scoped}"
    );
}

#[test]
fn the_baseline_composes_with_a_base_diff_run() {
    let dir = repo("with-base", BASELINE);
    std::fs::write(
        dir.join("docs/adr/ADR-1-record-1.md"),
        record(1, "Implemented", "a.rs").replace("Initial.", "Initial, edited."),
    )
    .unwrap();
    commit_all(&dir, "touch ADR-1");
    let parsed = json(&run(&dir, &["check", "--base", "HEAD~1"]));
    assert_eq!(parsed["baseline"]["ref"], "adoption", "{parsed}");
    assert!(
        !drift_findings(&parsed).contains(&"docs/adr/ADR-1-record-1.md".to_string()),
        "{parsed}"
    );
}

// ----------------------------------------------- pins: drift is judged at the ref

const A_V1: &str = "fn a() {}\n";
const A_V2: &str = "fn a() { let _ = 1; }\n";

fn sha256(content: &str) -> String {
    let path = common::TempFile::new("urzua-baseline-pin");
    std::fs::write(&path, content).unwrap();
    let out = Command::new("sha256sum").arg(&path).output().unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_string()
}

fn pinned_record(n: u8, pin: &str) -> String {
    format!(
        "---\nEmbodiment: 'Implemented'\nRealized-by: 'code:src/a.rs@sha256:{pin}'\n---\n# ADR-{n}: Record {n}\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
    )
}

/// `src/a.rs` is changed to V2 before the baseline is tagged and never after.
/// ADR-1 pins `pin_at_ref` when the tag is made; `a.rs` is untouched since, so
/// any drift a later edit of a record causes is not the file's.
fn pin_repo(name: &str, pin_at_ref: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-baseline-pin-{name}"));
    for d in ["docs/adr", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config(BASELINE)).unwrap();
    std::fs::write(dir.join("src/a.rs"), A_V1).unwrap();
    commit_all(&dir, "a v1");
    std::fs::write(dir.join("src/a.rs"), A_V2).unwrap();
    commit_all(&dir, "a v2");
    std::fs::write(
        dir.join("docs/adr/ADR-1-record-1.md"),
        pinned_record(1, pin_at_ref),
    )
    .unwrap();
    commit_all(&dir, "adr 1");
    git(&dir, &["tag", "adoption"]);
    dir
}

#[test]
fn a_pin_that_was_already_stale_at_the_ref_is_pre_existing() {
    let dir = pin_repo("stale-at-ref", &sha256(A_V1));
    let parsed = json(&run(&dir, &["check"]));
    assert_eq!(drift_findings(&parsed), Vec::<String>::new(), "{parsed}");
    assert_eq!(parsed["baseline"]["pre_existing"], 1, "{parsed}");
}

#[test]
fn a_pin_edited_after_the_ref_to_a_wrong_hash_is_introduced_drift() {
    let dir = pin_repo("edited", &sha256(A_V2));
    let clean = json(&run(&dir, &["check"]));
    assert_eq!(drift_findings(&clean), Vec::<String>::new(), "{clean}");
    std::fs::write(
        dir.join("docs/adr/ADR-1-record-1.md"),
        pinned_record(1, &"0".repeat(64)),
    )
    .unwrap();
    commit_all(&dir, "edit the pin");
    let out = run(&dir, &["check"]);
    let parsed = json(&out);
    assert_eq!(
        drift_findings(&parsed),
        ["docs/adr/ADR-1-record-1.md"],
        "{parsed}"
    );
    assert_eq!(out.status.code(), Some(1), "{parsed}");
    assert_eq!(parsed["baseline"]["pre_existing"], 0, "{parsed}");
    assert_eq!(parsed["baseline"]["introduced"], 1, "{parsed}");
}

#[test]
fn a_record_added_after_the_ref_citing_an_old_file_with_a_wrong_pin_is_introduced_drift() {
    let dir = pin_repo("added", &sha256(A_V2));
    std::fs::write(
        dir.join("docs/adr/ADR-2-record-2.md"),
        pinned_record(2, &sha256(A_V1)),
    )
    .unwrap();
    commit_all(&dir, "add adr 2");
    let out = run(&dir, &["check"]);
    let parsed = json(&out);
    assert_eq!(
        drift_findings(&parsed),
        ["docs/adr/ADR-2-record-2.md"],
        "{parsed}"
    );
    assert_eq!(out.status.code(), Some(1), "{parsed}");
    assert_eq!(parsed["baseline"]["introduced"], 1, "{parsed}");
}

#[test]
fn a_pin_renewed_after_the_ref_to_another_wrong_hash_is_introduced_drift() {
    // Stale at the ref (V1 against a V2 file), then renewed to a different wrong
    // pin: not the locator that was already drifted when the ref was tagged.
    let dir = pin_repo("renewed", &sha256(A_V1));
    std::fs::write(
        dir.join("docs/adr/ADR-1-record-1.md"),
        pinned_record(1, &sha256("fn a() { let _ = 99; }\n")),
    )
    .unwrap();
    commit_all(&dir, "renew the pin to the wrong content");
    let out = run(&dir, &["check"]);
    let parsed = json(&out);
    assert_eq!(
        drift_findings(&parsed),
        ["docs/adr/ADR-1-record-1.md"],
        "{parsed}"
    );
    assert_eq!(out.status.code(), Some(1), "{parsed}");
    assert_eq!(parsed["baseline"]["pre_existing"], 0, "{parsed}");
}
