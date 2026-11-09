//! ADR-97 amendment (2026-11-09, drift policy): `embodiment.claim-expired`'s
//! `drift` option sets, per locator category, whether drift since the
//! `Realized-by` line was last touched blocks a renewal (`block`, the default),
//! lets a reviewer's `--reason` renew it with the drift written into the
//! revision row (`acknowledge`), or leaves it out of renewal (`ignore`).
//! Real git fixtures, real binary, JSON only.

mod common;

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

fn date_days_ago(n: u32) -> String {
    let out = Command::new("date")
        .args(["-u", "-d", &format!("{n} days ago"), "+%Y-%m-%dT12:00:00Z"])
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn commit_all(dir: &Path, days_ago: u32) {
    git(dir, &["add", "-A"]);
    let date = date_days_ago(days_ago);
    let status = Command::new("git")
        .args(["commit", "-q", "-m", "fixture"])
        .env("GIT_AUTHOR_DATE", &date)
        .env("GIT_COMMITTER_DATE", &date)
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success());
}

fn run_urzua(dir: &Path, args: &[&str]) -> std::process::Output {
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

fn read(dir: &Path, rel: &str) -> String {
    std::fs::read_to_string(dir.join(rel)).unwrap()
}

const SRC: &str = "fn a() {}\n";
const TST: &str = "fn t() {}\n";
const PLAIN: &str = "plain\n";
const RECORD: &str = "docs/milestones/MILE-1-m.md";
const ADR: &str = "docs/adr/ADR-1-cited.md";

fn adr(rows: &str) -> String {
    format!(
        "---\nStatus: Accepted\n---\n# ADR-1: Cited\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n{rows}"
    )
}

fn config(drift: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n  embodiment.claim-expired:\n    level: warn\n    anchor_field: Last-audited\n    require_pin: [code, test]\n    max_batch: 3\n{drift}    periods:\n      - {{type: milestone, status: [Done], days: 30}}\n  embodiment.consistency: {{level: warn, drift_grace_days: 365}}\n\
         record_types:\n  \
         milestone:\n    dir: docs/milestones\n    prefix: MILE\n    header_shape: yaml-frontmatter\n    \
         required_fields: [Status]\n    known_fields: [Embodiment, Realized-by, Last-audited, Stable-Id]\n    \
         statuses:\n      values: [Planned, Done, Superseded]\n      success: [Done]\n      claim_retained: [Superseded]\n  \
         adr:\n    dir: docs/adr\n    prefix: ADR\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n"
    )
}

fn milestone(locators: &str) -> String {
    format!(
        "---\nStable-Id: 01AAAAAAAAAAAAAAAAAAAAAA01\nStatus: Done\nRealized-by: {locators}\n---\n# MILE-1: T\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
    )
}

/// An expired claim (40 days against 30) citing code, a test and `cited`, in a
/// clean fixture; the caller then changes what it wants to drift.
fn fixture(name: &str, drift: &str, cited: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-drift-policy-{name}"));
    for d in ["docs/milestones", "docs/adr", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config(drift)).unwrap();
    std::fs::write(dir.join("src/a.rs"), SRC).unwrap();
    std::fs::write(dir.join("src/t.rs"), TST).unwrap();
    std::fs::write(dir.join("docs/plain.md"), PLAIN).unwrap();
    std::fs::write(dir.join(ADR), adr("")).unwrap();
    let locators = format!("code:src/a.rs, test:src/t.rs, spec:{cited}");
    std::fs::write(dir.join(RECORD), milestone(&locators)).unwrap();
    commit_all(&dir, 40);
    dir
}

/// The cited ADR gains a revision row dated 2026-11-09 after the claim's line.
fn amend_adr(dir: &Path) {
    std::fs::write(
        dir.join(ADR),
        adr("> | 2026-11-09 | Re-pinned evidence. | **substantive** |\n"),
    )
    .unwrap();
    commit_all(dir, 1);
}

fn apply(dir: &Path, reason: &str) -> serde_json::Value {
    json(&run_urzua(
        dir,
        &[
            "reverify", "--apply", "--ids", RECORD, "--by", "ana", "--actor", "human", "--reason",
            reason,
        ],
    ))
}

fn refusals(parsed: &serde_json::Value) -> Vec<String> {
    parsed["records"][0]["refused"]
        .as_array()
        .map(|a| a.iter().map(|r| r.as_str().unwrap().to_string()).collect())
        .unwrap_or_default()
}

#[test]
fn a_spec_only_drift_under_acknowledge_renews_and_records_the_drift_observed_failing() {
    let dir = fixture(
        "ack",
        "    drift: {code: block, test: block, spec: acknowledge}\n",
        ADR,
    );
    amend_adr(&dir);

    // Detect lists the claim as renewable, with the drift shown.
    let detect = json(&run_urzua(&dir, &["reverify", "--ids", RECORD]));
    assert_eq!(detect["status"], "repairs-available", "{detect}");
    assert!(refusals(&detect).is_empty(), "{detect}");
    let shown = detect["records"][0]["drift_acknowledged"].to_string();
    assert!(
        shown.contains("ADR-1") && shown.contains("2026-11-09") && shown.contains("spec"),
        "{detect}"
    );

    let parsed = apply(&dir, "Read ADR-1's new row; the claim still holds.");
    assert_eq!(parsed["status"], "applied", "{parsed}");
    let text = read(&dir, RECORD);
    let row = text
        .lines()
        .find(|l| l.contains("| Re-verified ("))
        .unwrap_or_else(|| panic!("{text}"));
    assert!(
        row.contains("Drift acknowledged: spec:ADR-1 (latest revision 2026-11-09).")
            && row.contains("Reason: Read ADR-1's new row; the claim still holds."),
        "{row}"
    );
    assert!(text.contains("Last-audited: "), "{text}");
}

#[test]
fn a_spec_only_drift_under_block_or_undeclared_is_refused_naming_the_policy_observed_failing() {
    for (name, drift, says) in [
        ("blk", "    drift: {spec: block}\n", "is block"),
        ("und", "", "is not declared, so it is block"),
    ] {
        let dir = fixture(name, drift, ADR);
        amend_adr(&dir);
        let before = read(&dir, RECORD);
        let parsed = apply(&dir, "checked");
        assert_eq!(parsed["status"], "partial-failure", "{parsed}");
        let error = parsed["failed"][0]["error"].as_str().unwrap();
        assert!(
            error.contains("'spec'")
                && error.contains(says)
                && error.contains("drift: {spec: acknowledge}")
                && error.contains("--pin-only` cannot"),
            "{error}"
        );
        assert!(!error.contains("re-pin an unpinned one"), "{error}");
        assert_eq!(read(&dir, RECORD), before);
        let detect = json(&run_urzua(&dir, &["reverify", "--ids", RECORD]));
        assert_eq!(detect["status"], "ok", "{detect}");
        assert!(refusals(&detect)[0].contains("'spec'"), "{detect}");
    }
}

#[test]
fn a_code_drift_is_still_refused_when_only_spec_acknowledges_observed_failing() {
    let dir = fixture("code", "    drift: {code: block, spec: acknowledge}\n", ADR);
    std::fs::write(dir.join("src/a.rs"), "fn a() { let _ = 1; }\n").unwrap();
    commit_all(&dir, 1);
    let before = read(&dir, RECORD);
    let parsed = apply(&dir, "checked");
    let error = parsed["failed"][0]["error"].as_str().unwrap();
    assert!(
        error.contains("reverify --pin-only") && error.contains("fix --rebaseline"),
        "{error}"
    );
    assert_eq!(read(&dir, RECORD), before);
}

#[test]
fn an_ignored_category_renews_without_listing_its_drift_observed_failing() {
    let dir = fixture("ign", "    drift: {spec: ignore}\n", ADR);
    amend_adr(&dir);
    let detect = json(&run_urzua(&dir, &["reverify", "--ids", RECORD]));
    assert!(refusals(&detect).is_empty(), "{detect}");
    assert!(
        detect["records"][0].get("drift_acknowledged").is_none(),
        "{detect}"
    );
    let parsed = apply(&dir, "The cited ADR is judged by its own rules.");
    assert_eq!(parsed["status"], "applied", "{parsed}");
    let text = read(&dir, RECORD);
    assert!(!text.contains("Drift acknowledged"), "{text}");
    assert!(text.contains("| Re-verified ("), "{text}");
}

#[test]
fn a_cited_file_that_is_not_a_record_is_acknowledged_by_path_and_hash_prefix_observed_failing() {
    let dir = fixture("file", "    drift: {spec: acknowledge}\n", "docs/plain.md");
    std::fs::write(dir.join("docs/plain.md"), "changed\n").unwrap();
    commit_all(&dir, 1);
    let parsed = apply(&dir, "Read the one-line change to docs/plain.md.");
    assert_eq!(parsed["status"], "applied", "{parsed}");
    let text = read(&dir, RECORD);
    let hash = {
        let out = Command::new("sh")
            .args(["-c", "printf 'changed\\n' | sha256sum"])
            .output()
            .unwrap();
        String::from_utf8(out.stdout).unwrap()[..12].to_string()
    };
    assert!(
        text.contains(&format!(
            "Drift acknowledged: spec:docs/plain.md@sha256:{hash}."
        )),
        "{text}"
    );
}

#[test]
fn an_unknown_category_or_policy_is_a_load_error_naming_the_valid_ones_observed_failing() {
    for (drift, needles) in [
        (
            "    drift: {nope: block}\n",
            vec!["'drift'", "'nope'", "test", "code", "spec"],
        ),
        (
            "    drift: {spec: allow}\n",
            vec!["allow", "block", "acknowledge", "ignore"],
        ),
    ] {
        let dir = fixture("bad", drift, ADR);
        let out = run_urzua(&dir, &["reverify"]);
        assert_eq!(out.status.code(), Some(2), "{out:?}");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        for needle in needles {
            assert!(text.contains(needle), "{needle}: {text}");
        }
    }
}

#[test]
fn drift_is_a_load_error_on_any_other_rule() {
    let dir = fixture("other", "", ADR);
    let cfg = read(&dir, ".urzua/config.yaml").replace(
        "embodiment.consistency: {level: warn, drift_grace_days: 365}",
        "embodiment.consistency: {level: warn, drift: {spec: ignore}}",
    );
    std::fs::write(dir.join(".urzua/config.yaml"), cfg).unwrap();
    let out = run_urzua(&dir, &["check"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
}
