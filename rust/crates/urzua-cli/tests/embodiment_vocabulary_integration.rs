//! The declared `Embodiment` vocabulary (`MILE-144`) against the real binary:
//! a repository that names its own states is held to those names by `check`
//! and repaired to them by `fix`, and a bad declaration is refused at load.

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

fn run_urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

const VOCABULARY: &str = "embodiment:\n  drift: Stale\n  tiers:\n    - {state: Proven, evidence: test, claims_built: true}\n    - {state: Built, evidence: code, claims_built: true}\n    - {state: Described, evidence: spec}\n  none: Untouched\n";

/// ADR-1 states `Described` on spec evidence (agrees), ADR-2 states `Proven`
/// on spec evidence (disagrees), ADR-3 states `Built` with no evidence at all
/// (a claim with nothing behind it), ADR-4 states the built-in `Verified`,
/// which this vocabulary does not declare.
fn repo(name: &str, vocabulary: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-vocab-{name}"));
    for d in ["docs/adr", "docs/spec", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\n{vocabulary}rules:\n  embodiment.consistency:\n    level: error\n  \
             embodiment.requires-accepted-status: error\n\
             record_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    \
             required_fields: [\"Status\", \"Embodiment\"]\n    known_fields: [\"Realized-by\"]\n    \
             header_shape: \"yaml-frontmatter\"\n    \
             statuses:\n      values: [Proposed, Accepted]\n      success: [Accepted]\n"
        ),
    )
    .unwrap();
    for (n, state, locator) in [
        (1, "Described", "Realized-by: spec:docs/spec/s.md\n"),
        (2, "Proven", "Realized-by: spec:docs/spec/s.md\n"),
        (3, "Built", ""),
        (4, "Verified", "Realized-by: spec:docs/spec/s.md\n"),
    ] {
        std::fs::write(
            dir.join(format!("docs/adr/ADR-{n}-x.md")),
            format!("---\nStatus: Accepted\nEmbodiment: {state}\n{locator}---\n# {n} — X\n"),
        )
        .unwrap();
    }
    std::fs::write(dir.join("docs/spec/s.md"), "spec\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn parsed(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).expect("json")
}

fn consistency(report: &serde_json::Value) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "embodiment.consistency")
        .map(|f| {
            (
                f["file"].as_str().unwrap().to_string(),
                f["message"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    found.sort();
    found
}

#[test]
fn a_declared_vocabulary_is_what_check_holds_records_to() {
    let dir = repo("check", VOCABULARY);
    let found = consistency(&parsed(&run_urzua(&dir, &["check"])));
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(
        found[0].0.contains("ADR-2") && found[0].1.contains("'Described'"),
        "{found:?}"
    );
    assert!(
        found[1].0.contains("ADR-3") && found[1].1.contains("'Untouched'"),
        "{found:?}"
    );
    assert!(
        found[2].0.contains("ADR-4") && found[2].1.contains("'Described'"),
        "{found:?}"
    );
}

#[test]
fn without_a_declaration_the_built_in_vocabulary_applies() {
    let dir = repo("default", "");
    let found = consistency(&parsed(&run_urzua(&dir, &["check"])));
    // ADR-3 states `Built`, which the built-in vocabulary does not know as a
    // claim, so it is not judged; the other three disagree with `Specified`.
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(
        found.iter().all(|(_, m)| m.contains("'Specified'")),
        "{found:?}"
    );
}

#[test]
fn stating_the_default_vocabulary_explicitly_changes_nothing() {
    let default_decl = "embodiment:\n  drift: Drift detected\n  tiers:\n    - {state: Verified, evidence: test, claims_built: true}\n    - {state: Implemented, evidence: code, claims_built: true}\n    - {state: Specified, evidence: spec}\n  none: Not started\n";
    let (a, b) = (repo("implicit", ""), repo("explicit", default_decl));
    let text = |dir: &Path| {
        String::from_utf8_lossy(&run_urzua(dir, &["check"]).stdout)
            .replace(dir.to_str().unwrap(), "<dir>")
    };
    assert_eq!(text(&a), text(&b));
}

#[test]
fn fix_repairs_to_the_declared_names() {
    let dir = repo("fix", VOCABULARY);
    let report = parsed(&run_urzua(&dir, &["fix"]));
    let repairs = report["repairs"].as_array().unwrap();
    let computed: Vec<(String, String)> = repairs
        .iter()
        .map(|r| {
            (
                r["record"].as_str().unwrap().to_string(),
                r["computed_value"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(computed.len(), 2, "{report}");
    assert!(
        computed
            .iter()
            .any(|(r, v)| r.contains("ADR-2") && v == "Described"),
        "{computed:?}"
    );
    assert!(
        computed
            .iter()
            .any(|(r, v)| r.contains("ADR-4") && v == "Described"),
        "{computed:?}"
    );
}

#[test]
fn a_built_claiming_declared_state_needs_an_accepted_status() {
    let dir = repo("accepted", VOCABULARY);
    std::fs::write(
        dir.join("docs/adr/ADR-3-x.md"),
        "---\nStatus: Proposed\nEmbodiment: Proven\n---\n# 3 — X\n",
    )
    .unwrap();
    let report = parsed(&run_urzua(&dir, &["check"]));
    let hit = report["findings"].as_array().unwrap().iter().any(|f| {
        f["rule"] == "embodiment.requires-accepted-status"
            && f["file"].as_str().unwrap().contains("ADR-3")
    });
    assert!(hit, "{report}");
}

#[test]
fn a_bad_declaration_is_refused_at_load() {
    for (name, decl, needle) in [
        ("dup", "embodiment:\n  drift: A\n  tiers: [{state: A, evidence: test, claims_built: true}]\n  none: B\n", "twice"),
        ("ev", "embodiment:\n  drift: A\n  tiers: [{state: B, evidence: test, claims_built: true}, {state: C, evidence: test}]\n  none: D\n", "evidence"),
        ("empty", "embodiment:\n  drift: A\n  tiers: []\n  none: D\n", "tiers"),
        (
            "claims",
            "embodiment:\n  drift: A\n  tiers: [{state: B, evidence: test}]\n  none: D\n",
            "claims_built",
        ),
    ] {
        let dir = repo(name, decl);
        let out = run_urzua(&dir, &["check"]);
        assert_eq!(out.status.code(), Some(2));
        let text = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        assert!(text.contains("embodiment") && text.contains(needle), "{name}: {text}");
    }
}

#[test]
fn a_sticky_state_naming_a_declared_state_is_refused() {
    let dir = repo("sticky", VOCABULARY);
    let cfg = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        cfg.replace(
            "level: error\n  embodiment.requires",
            "level: error\n    sticky_states: [Proven]\n  embodiment.requires",
        ),
    )
    .unwrap();
    let out = run_urzua(&dir, &["check"]);
    assert_eq!(out.status.code(), Some(2));
    let text =
        String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    assert!(
        text.contains("sticky_states") && text.contains("'Proven'"),
        "{text}"
    );
}

/// A repository of `adr` records `(number, stated state, Realized-by)` under
/// `vocabulary`, with `rules` appended to the rule table.
fn custom_repo(
    name: &str,
    vocabulary: &str,
    rules: &str,
    records: &[(u32, &str, &str)],
) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-vocab-{name}"));
    for d in ["docs/adr", "docs/spec", "src", "tests", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\n{vocabulary}rules:\n  embodiment.consistency: error\n{rules}\
             record_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    \
             required_fields: [\"Status\", \"Embodiment\"]\n    known_fields: [\"Realized-by\"]\n    \
             header_shape: \"yaml-frontmatter\"\n    \
             statuses:\n      values: [Planned, Done]\n      success: [Done]\n{DELIVERY_DECL}"
        ),
    )
    .unwrap();
    for (n, state, locator) in records {
        let locator = if locator.is_empty() {
            String::new()
        } else {
            format!("Realized-by: {locator}\n")
        };
        std::fs::write(
            dir.join(format!("docs/adr/ADR-{n}-x.md")),
            format!("---\nStatus: Planned\nEmbodiment: {state}\n{locator}---\n# {n} — X\n"),
        )
        .unwrap();
    }
    std::fs::write(dir.join("docs/spec/s.md"), "spec\n").unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join("tests/a.rs"), "fn t() {}\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

/// A delivery declaration written in the renamed vocabulary.
const DELIVERY_DECL: &str = "      delivery:\n        delivered_tiers: [Built, Proven]\n        unevidenced_tiers: [Untouched]\n        not_done: [Planned]\n        set_status: Done\n";

fn findings_of<'a>(report: &'a serde_json::Value, rule: &str) -> Vec<&'a serde_json::Value> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == rule)
        .collect()
}

#[test]
fn delivery_agreement_reads_the_renamed_tiers_end_to_end() {
    // Planned with code evidence computes to `Built`, a delivered tier.
    let dir = custom_repo(
        "delivery",
        VOCABULARY,
        "  status.delivery-agreement: error\n",
        &[(1, "Built", "code:src/a.rs")],
    );
    let report = parsed(&run_urzua(&dir, &["check"]));
    let found = findings_of(&report, "status.delivery-agreement");
    assert_eq!(found.len(), 1, "{report}");
    assert!(
        found[0]["message"].as_str().unwrap().contains("Built"),
        "{report}"
    );
    // `fix` writes the declared `set_status`.
    let fixed = parsed(&run_urzua(&dir, &["fix"]));
    let repairs = fixed["repairs"].as_array().unwrap();
    assert!(
        repairs.iter().any(|r| r["computed_value"] == "Done"),
        "{fixed}"
    );
}

#[test]
fn a_delivery_tier_in_the_built_in_vocabulary_is_refused_under_a_renamed_one() {
    let dir = custom_repo(
        "delivery-bad",
        VOCABULARY,
        "",
        &[(1, "Built", "code:src/a.rs")],
    );
    let cfg = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        cfg.replace(
            "delivered_tiers: [Built, Proven]",
            "delivered_tiers: [Implemented]",
        ),
    )
    .unwrap();
    let out = run_urzua(&dir, &["check"]);
    assert_eq!(out.status.code(), Some(2));
    let text =
        String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    assert!(
        text.contains("Implemented") && text.contains("delivered_tiers"),
        "{text}"
    );
}

#[test]
fn the_declared_tier_order_decides_which_evidence_wins() {
    // Code outranks test here, the reverse of the built-in order.
    let reordered = "embodiment:\n  drift: Stale\n  tiers:\n    - {state: Built, evidence: code, claims_built: true}\n    - {state: Proven, evidence: test, claims_built: true}\n    - {state: Described, evidence: spec}\n  none: Untouched\n";
    let both = "code:src/a.rs, test:tests/a.rs";
    let dir = custom_repo(
        "order",
        reordered,
        "",
        &[(1, "Built", both), (2, "Proven", both)],
    );
    let found = consistency(&parsed(&run_urzua(&dir, &["check"])));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].0.contains("ADR-2") && found[0].1.contains("'Built'"),
        "{found:?}"
    );
}

#[test]
fn a_declaration_missing_a_key_is_refused_not_defaulted() {
    for (name, decl, needle) in [
        ("nonekey", "embodiment:\n  drift: A\n  tiers: [{state: B, evidence: test, claims_built: true}]\n", "none"),
        ("driftkey", "embodiment:\n  tiers: [{state: B, evidence: test, claims_built: true}]\n  none: D\n", "drift"),
        ("tierskey", "embodiment:\n  drift: A\n  none: D\n", "tiers"),
        ("stateless", "embodiment:\n  drift: A\n  tiers: [{evidence: test, claims_built: true}]\n  none: D\n", "state"),
        ("evidencekey", "embodiment:\n  drift: A\n  tiers: [{state: B, claims_built: true}]\n  none: D\n", "evidence"),
        ("novalue", "embodiment:\n", "embodiment"),
        ("unknown", "embodiment:\n  drift: A\n  colour: red\n  tiers: [{state: B, evidence: test, claims_built: true}]\n  none: D\n", "colour"),
    ] {
        let dir = repo(name, decl);
        let out = run_urzua(&dir, &["check"]);
        assert_eq!(out.status.code(), Some(2), "{name}: loaded");
        let text = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        assert!(text.contains(needle), "{name}: {text}");
    }
}
