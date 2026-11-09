//! ADR-97 drift-policy amendment, consistency fix: under `drift: acknowledge`
//! a drift that has outlived `drift_grace_days` must not make `reverify`
//! refuse the record ("run `urzua fix` first"), and the acknowledged renewal
//! must leave `embodiment.consistency` with nothing to report for the
//! acknowledged locators. Real git fixtures, real binary, JSON only.

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

fn sha256_of(content: &str) -> String {
    let out = Command::new("sh")
        .args(["-c", &format!("printf '{content}' | sha256sum")])
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap()[..64].to_string()
}

const RECORD: &str = "docs/adr/ADR-2-claim.md";
const CITED: &str = "docs/adr/ADR-1-cited.md";
const SRC: &str = "fn a() {}\n";

fn adr(rows: &str) -> String {
    format!(
        "---\nStatus: Accepted\n---\n# ADR-1: Cited\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n{rows}"
    )
}

fn config(drift: &str, audit_field: bool) -> String {
    let audit = if audit_field {
        ", drift_audit_field: Last-audited"
    } else {
        ""
    };
    format!(
        "schema_version: 2\nrules:\n  embodiment.claim-expired:\n    level: warn\n    anchor_field: Last-audited\n    require_pin: [code]\n{drift}    periods:\n      - {{type: adr, status: [Accepted], days: 30}}\n  embodiment.consistency: {{level: error, drift_grace_days: 7{audit}}}\n\
         record_types:\n  adr:\n    dir: docs/adr\n    prefix: ADR\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n    known_fields: [Embodiment, Realized-by, Last-audited, Stable-Id]\n    statuses:\n      values: [Accepted]\n      success: [Accepted]\n"
    )
}

fn claim(locators: &str, extra: &str) -> String {
    format!(
        "---\nStable-Id: 01AAAAAAAAAAAAAAAAAAAAAA02\nStatus: Accepted\nEmbodiment: Implemented\nRealized-by: {locators}\n---\n# ADR-2: Claim\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n{extra}"
    )
}

/// A claim made 40 days ago (30-day period, so expired) over a pinned `code:`
/// locator and a `spec:` locator; the cited file `what` then changed 20 days
/// ago, 13 days past the 7-day grace window.
fn drifted(name: &str, drift: &str, audit_field: bool, what: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-drift-grace-{name}"));
    for d in ["docs/adr", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config(drift, audit_field)).unwrap();
    std::fs::write(dir.join("src/a.rs"), SRC).unwrap();
    std::fs::write(dir.join(CITED), adr("")).unwrap();
    let locators = format!(
        "code:src/a.rs@sha256:{}, spec:{CITED}",
        sha256_of("fn a() {}\\n")
    );
    std::fs::write(dir.join(RECORD), claim(&locators, "")).unwrap();
    commit_all(&dir, 40);
    if what == "code" {
        std::fs::write(dir.join("src/a.rs"), "fn a() { let _ = 1; }\n").unwrap();
    } else {
        std::fs::write(
            dir.join(CITED),
            adr("> | 2026-11-01 | Amended. | **substantive** |\n"),
        )
        .unwrap();
    }
    commit_all(&dir, 20);
    dir
}

fn renew(dir: &Path) -> serde_json::Value {
    json(&run_urzua(
        dir,
        &[
            "reverify",
            "--apply",
            "--ids",
            RECORD,
            "--by",
            "ana",
            "--actor",
            "human",
            "--reason",
            "Read the change; the claim still holds.",
        ],
    ))
}

fn refusals(parsed: &serde_json::Value) -> Vec<String> {
    parsed["records"][0]["refused"]
        .as_array()
        .map(|a| a.iter().map(|r| r.as_str().unwrap().to_string()).collect())
        .unwrap_or_default()
}

fn consistency_errors(dir: &Path) -> String {
    if std::env::var("DBG").is_ok() {
        let o = run_urzua(dir, &["check"]);
        eprintln!(
            "{}\n{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
    }
    let out = run_urzua(dir, &["check", "--rules", "embodiment.consistency"]);
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn a_pinned_code_drift_past_grace_renews_and_check_is_clean_observed_failing() {
    let dir = drifted("code", "    drift: {code: acknowledge}\n", true, "code");
    assert!(consistency_errors(&dir).contains("Drift detected"));

    let detect = json(&run_urzua(&dir, &["reverify", "--ids", RECORD]));
    assert!(refusals(&detect).is_empty(), "{detect}");

    let parsed = renew(&dir);
    assert_eq!(parsed["status"], "applied", "{parsed}");
    let out = consistency_errors(&dir);
    assert!(!out.contains("Drift detected"), "{out}");
    let check = run_urzua(&dir, &["check", "--rules", "embodiment.consistency"]);
    assert_eq!(check.status.code(), Some(0), "{check:?}");
}

#[test]
fn an_unpinned_spec_drift_past_grace_renews_and_stays_cleared_observed_failing() {
    // No `drift_audit_field`: the grace window counts from the cited record's
    // own last change, so only the acknowledgement can clear the drift.
    let dir = drifted("spec", "    drift: {spec: acknowledge}\n", false, "spec");
    assert!(consistency_errors(&dir).contains("Drift detected"));

    let detect = json(&run_urzua(&dir, &["reverify", "--ids", RECORD]));
    assert!(refusals(&detect).is_empty(), "{detect}");

    let parsed = renew(&dir);
    assert_eq!(parsed["status"], "applied", "{parsed}");
    let out = consistency_errors(&dir);
    assert!(!out.contains("Drift detected"), "{out}");
    let check = run_urzua(&dir, &["check", "--rules", "embodiment.consistency"]);
    assert_eq!(check.status.code(), Some(0), "{check:?}");
}

/// The row `reverify` writes for a renewal that acknowledged `names`, dated
/// `day`, in the tool's exact shape.
fn tool_row(day: &str, evidence: &str, names: &str) -> String {
    format!(
        "> | {day} | Re-verified (attested by ana, identity: asserted, actor: human): 2 locators re-resolved, 1 hash-checked, 1 spec locators checked by git history only. Evidence: {evidence}; Drift acknowledged: {names}. Reason: r **Why:** the claim was 40 days old against a 30-day period. | **structural** |\n"
    )
}

const EVIDENCE: &str = "src/a.rs@sha256:5a1c2f0e9b8d, docs/adr/ADR-1-cited.md";

/// An acknowledging row written 15 days ago naming `names`.
fn acknowledge_15_days_ago_naming(dir: &Path, names: &str) {
    let day = date_days_ago(15)[..10].to_string();
    let text = std::fs::read_to_string(dir.join(RECORD)).unwrap();
    std::fs::write(
        dir.join(RECORD),
        format!("{text}{}", tool_row(&day, EVIDENCE, names)),
    )
    .unwrap();
    commit_all(dir, 15);
}

/// An acknowledging row written 15 days ago, the way `reverify` writes one.
fn acknowledge_15_days_ago(dir: &Path) {
    acknowledge_15_days_ago_naming(dir, "spec:ADR-1 (latest revision 2026-11-01)");
}

#[test]
fn an_acknowledged_change_stays_cleared_and_a_later_one_is_drift_again() {
    let dir = drifted("later", "    drift: {spec: acknowledge}\n", false, "spec");
    acknowledge_15_days_ago(&dir);
    // The change 20 days ago predates the row: acknowledged.
    assert!(
        !consistency_errors(&dir).contains("Drift detected"),
        "{}",
        consistency_errors(&dir)
    );
    // A change 10 days ago, past the 7-day grace, postdates it.
    std::fs::write(
        dir.join(CITED),
        adr("> | 2026-11-01 | Amended. | **substantive** |\n> | 2026-11-10 | Again. | **substantive** |\n"),
    )
    .unwrap();
    commit_all(&dir, 10);
    assert!(consistency_errors(&dir).contains("Drift detected"));
}

#[test]
fn under_block_or_undeclared_the_disagreement_refusal_and_the_drift_stay() {
    for (name, drift) in [("blk", "    drift: {spec: block}\n"), ("und", "")] {
        let dir = drifted(name, drift, true, "spec");
        let detect = json(&run_urzua(&dir, &["reverify", "--ids", RECORD]));
        let said = refusals(&detect).join(" | ");
        assert!(said.contains("run `urzua fix` first"), "{said}");
        assert!(said.contains("'spec'"), "{said}");
        assert!(consistency_errors(&dir).contains("Drift detected"));
    }
}

#[test]
fn an_undeclared_policy_ignores_an_acknowledging_row_on_the_record() {
    let dir = drifted("row", "", false, "spec");
    acknowledge_15_days_ago(&dir);
    assert!(consistency_errors(&dir).contains("Drift detected"));
}

#[test]
fn a_hand_typed_row_without_the_tools_shape_clears_nothing_observed_failing() {
    let dir = drifted("typed", "    drift: {spec: acknowledge}\n", false, "spec");
    let day = date_days_ago(15)[..10].to_string();
    let text = std::fs::read_to_string(dir.join(RECORD)).unwrap();
    std::fs::write(
        dir.join(RECORD),
        format!(
            "{text}> | {day} | Re-verified (attested by ana (human)): looked. Evidence: x; Drift acknowledged: spec:ADR-1. Reason: r | **structural** |\n"
        ),
    )
    .unwrap();
    commit_all(&dir, 15);
    assert!(consistency_errors(&dir).contains("Drift detected"));
}

/// Two cited records, both amended 20 days ago, and a claim over both.
fn two_drifted(name: &str) -> common::TempDir {
    let dir = drifted(name, "    drift: {spec: acknowledge}\n", false, "spec");
    let other = "docs/adr/ADR-3-other.md";
    let claim_text = std::fs::read_to_string(dir.join(RECORD)).unwrap();
    std::fs::write(dir.join(RECORD), claim_text).unwrap();
    // The second cited record exists from the start of the claim: rebuild the
    // history so both changes postdate the claim's line.
    std::fs::write(dir.join(other), adr("")).unwrap();
    let text = std::fs::read_to_string(dir.join(RECORD)).unwrap();
    std::fs::write(
        dir.join(RECORD),
        text.replace(
            &format!("spec:{CITED}"),
            &format!("spec:{CITED}, spec:{other}"),
        ),
    )
    .unwrap();
    commit_all(&dir, 40);
    std::fs::write(
        dir.join(other),
        adr("> | 2026-11-01 | Amended. | **substantive** |\n"),
    )
    .unwrap();
    commit_all(&dir, 20);
    dir
}

#[test]
fn a_row_clears_only_the_locators_it_names_observed_failing() {
    let dir = two_drifted("named");
    assert!(consistency_errors(&dir).contains("Drift detected"));
    acknowledge_15_days_ago(&dir);
    // ADR-1 is named and cleared; ADR-3 changed the same day and is not named.
    let out = consistency_errors(&dir);
    assert!(out.contains("Drift detected"), "{out}");
    // Naming both clears both.
    acknowledge_15_days_ago_naming(
        &dir,
        "spec:ADR-1 (latest revision 2026-11-01), spec:ADR-3 (latest revision 2026-11-01)",
    );
    let out = consistency_errors(&dir);
    assert!(!out.contains("Drift detected"), "{out}");
}

#[test]
fn a_change_on_the_rows_own_day_is_acknowledged_whatever_the_hour() {
    // Day granularity: the row clears a change made later the same day. The
    // caveat ADR-97 item 6 states.
    let dir = drifted("sameday", "    drift: {spec: acknowledge}\n", false, "spec");
    let today = date_days_ago(0)[..10].to_string();
    let text = std::fs::read_to_string(dir.join(RECORD)).unwrap();
    std::fs::write(
        dir.join(RECORD),
        format!(
            "{text}{}",
            tool_row(&today, EVIDENCE, "spec:ADR-1 (latest revision 2026-11-01)")
        ),
    )
    .unwrap();
    commit_all(&dir, 0);
    std::fs::write(
        dir.join(CITED),
        adr("> | 2026-11-01 | Amended. | **substantive** |\n> | 2026-11-10 | Later the same day. | **substantive** |\n"),
    )
    .unwrap();
    commit_all(&dir, 0);
    let out = consistency_errors(&dir);
    assert!(!out.contains("Drift detected"), "{out}");
}

const RENEWAL: &str = "embodiment.claim-renewal-unattested";

fn renewal_findings(dir: &Path) -> Vec<String> {
    let parsed = json(&run_urzua(
        dir,
        &["check", "--base", "base", "--rules", RENEWAL],
    ));
    parsed["findings"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|f| f["rule"] == RENEWAL)
                .map(|f| f["message"].as_str().unwrap().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn with_renewal_rule(dir: &Path) {
    let cfg = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        cfg.replace("rules:\n", &format!("rules:\n  {RENEWAL}: warn\n")),
    )
    .unwrap();
    commit_all(dir, 25);
}

#[test]
fn a_genuine_tool_row_passes_and_a_row_naming_the_wrong_locator_is_reported_observed_failing() {
    let dir = drifted("diff", "    drift: {spec: acknowledge}\n", false, "spec");
    with_renewal_rule(&dir);
    git(&dir, &["tag", "base"]);
    let parsed = renew(&dir);
    assert_eq!(parsed["status"], "applied", "{parsed}");
    commit_all(&dir, 0);
    assert_eq!(renewal_findings(&dir), Vec::<String>::new());

    // The same row naming a record that had not drifted.
    let text = std::fs::read_to_string(dir.join(RECORD)).unwrap();
    assert!(text.contains("Drift acknowledged: spec:ADR-1"), "{text}");
    std::fs::write(
        dir.join(RECORD),
        text.replace(
            "Drift acknowledged: spec:ADR-1",
            "Drift acknowledged: spec:ADR-9",
        ),
    )
    .unwrap();
    commit_all(&dir, 0);
    let found = renewal_findings(&dir);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("spec:ADR-9"), "{found:?}");

    // The same row listing evidence that is not the record's locators.
    std::fs::write(
        dir.join(RECORD),
        text.replace("Evidence: ", "Evidence: src/other.rs, "),
    )
    .unwrap();
    commit_all(&dir, 0);
    let found = renewal_findings(&dir);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("evidence"), "{found:?}");
}
