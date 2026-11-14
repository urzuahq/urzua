//! MILE-72: every command's JSON carries a top-level `next_action` (a bare
//! command or `null`) and `next_action_reason` (a sentence or `null`, present
//! exactly when `next_action` is). Real git-repo fixtures, real binary.
//!
//! Verifies: ADR-23, ADR-46

mod common;

use std::path::Path;

fn run(dir: &Path, args: &[&str]) -> serde_json::Value {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

const CONFIG: &str = "schema_version: 2\nrules: {embodiment.consistency: error, header.required-fields: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Embodiment\", \"Realized-by\"]\n";

const DRIFTED: &str = "---\nStatus: Accepted\nEmbodiment: Implemented\nRealized-by: code:src/a.rs\n---\n# ADR-1: First\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n";
const NO_STATUS: &str = "---\nDate: 2026-01-01\n---\n# ADR-2: Second\n";

/// ADR-1 drifted (a finding `urzua fix --apply` repairs); ADR-2 lacks `Status`
/// (a finding only a human can fix) unless `judgment_only` drops ADR-1's drift.
fn fixture(name: &str, with_drift: bool, with_judgment: bool) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-next-action-{name}"));
    for sub in ["docs/adr", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    common::git(&dir, &["init", "-q"]);
    common::git(&dir, &["config", "user.email", "test@example.com"]);
    common::git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    if with_drift {
        std::fs::write(dir.join("docs/adr/ADR-1-first.md"), DRIFTED).unwrap();
    }
    if with_judgment {
        std::fs::write(dir.join("docs/adr/ADR-2-second.md"), NO_STATUS).unwrap();
    }
    common::git(&dir, &["add", "-A"]);
    common::git(&dir, &["commit", "-q", "-m", "fixture"]);
    if with_drift {
        std::fs::write(dir.join("src/a.rs"), "fn a() { let _ = 1; }\n").unwrap();
        common::git(&dir, &["add", "-A"]);
        common::git(&dir, &["commit", "-q", "-m", "drift"]);
    }
    dir
}

fn pair(v: &serde_json::Value) -> (&serde_json::Value, &serde_json::Value) {
    (
        v.get("next_action")
            .unwrap_or_else(|| panic!("no next_action: {v}")),
        v.get("next_action_reason")
            .unwrap_or_else(|| panic!("no next_action_reason: {v}")),
    )
}

#[test]
fn a_mixed_fixable_and_judgment_check_suggests_urzua_fix() {
    let dir = fixture("mixed", true, true);
    let v = run(&dir, &["check"]);
    let (action, reason) = pair(&v);
    assert_eq!(
        action, "urzua fix --apply --ids docs/adr/ADR-1-first.md",
        "{v}"
    );
    assert_eq!(reason, "repairs 1 of 2 findings", "{v}");
}

#[test]
fn a_check_whose_only_blocker_needs_a_human_has_a_null_pair() {
    let dir = fixture("judgment", false, true);
    let v = run(&dir, &["check"]);
    assert!(
        v["findings"].as_array().is_some_and(|f| !f.is_empty()),
        "{v}"
    );
    assert_eq!(
        pair(&v),
        (&serde_json::Value::Null, &serde_json::Value::Null)
    );
}

#[test]
fn a_clean_check_has_a_null_pair() {
    let dir = fixture("clean", false, false);
    let v = run(&dir, &["check"]);
    assert_eq!(
        pair(&v),
        (&serde_json::Value::Null, &serde_json::Value::Null)
    );
}

#[test]
fn audit_derives_the_same_action_as_check() {
    let dir = fixture("audit", true, true);
    let v = run(&dir, &["audit"]);
    assert_eq!(
        pair(&v).0,
        "urzua fix --apply --ids docs/adr/ADR-1-first.md",
        "{v}"
    );
}

#[test]
fn fix_suggests_applying_then_checking() {
    let dir = fixture("fix", true, false);
    let detect = run(&dir, &["fix"]);
    assert_eq!(detect["status"], "repairs-available", "{detect}");
    assert_eq!(
        pair(&detect).0,
        "urzua fix --apply --ids docs/adr/ADR-1-first.md",
        "{detect}"
    );
    assert_eq!(pair(&detect).1, "writes 1 repair", "{detect}");
    let applied = run(&dir, &["fix", "--apply", "--force", "--by", "me"]);
    assert_eq!(applied["status"], "applied", "{applied}");
    assert_eq!(pair(&applied).0, "urzua check", "{applied}");
    assert_eq!(
        pair(&applied).1,
        "re-checks the corpus after 1 write",
        "{applied}"
    );
    let again = run(&dir, &["fix"]);
    assert_eq!(
        pair(&again),
        (&serde_json::Value::Null, &serde_json::Value::Null)
    );
}

#[test]
fn every_other_command_and_a_could_not_run_emit_both_keys_as_null() {
    let dir = fixture("others", true, true);
    let outputs = [
        run(&dir, &["graph"]),
        run(&dir, &["rules"]),
        run(&dir, &["doctor"]),
        run(&dir, &["export", "--format", "json"]),
        run(&dir, &["explain", "docs/adr/ADR-1-first.md"]),
        run(&dir, &["index"]),
        run(&dir, &["check", "--rules", "no.such-rule"]),
    ];
    for v in outputs {
        assert_eq!(
            pair(&v),
            (&serde_json::Value::Null, &serde_json::Value::Null),
            "{v}"
        );
    }
    // Outside any repository there is no config: still JSON, still both keys.
    let empty = common::TempDir::new("urzua-next-action-no-config");
    let v = run(&empty, &["check"]);
    assert_eq!(v["status"], "not-run", "{v}");
    assert_eq!(
        pair(&v),
        (&serde_json::Value::Null, &serde_json::Value::Null)
    );
}

/// The guard that lets `emit` flatten a report next to the two keys
/// (ADR-46's amendment of 2026-11-14): a report that declared either key
/// itself would print it twice. Every command's raw stdout has each key
/// exactly once at the top level.
#[test]
fn every_report_prints_each_next_action_key_exactly_once() {
    let dir = fixture("once", true, true);
    let runs: &[&[&str]] = &[
        &["check"],
        &["audit"],
        &["fix"],
        &["fix", "--rebaseline"],
        &["graph"],
        &["rules"],
        &["doctor"],
        &["export", "--format", "json"],
        &["explain", "docs/adr/ADR-1-first.md"],
        &["explain", "--rule", "header.required-fields"],
        &["index"],
        &["roadmap"],
        &["reverify"],
        &["migrate", "ids"],
        &["migrate", "filenames"],
        &["check", "--rules", "no.such-rule"],
    ];
    for args in runs {
        let out = common::urzua()
            .args(*args)
            .current_dir(&dir)
            .output()
            .expect("urzua binary should run");
        let text = String::from_utf8(out.stdout).unwrap();
        for key in ["next_action", "next_action_reason"] {
            let needle = format!("\n  \"{key}\":");
            assert_eq!(text.matches(&needle).count(), 1, "{args:?} {key}: {text}");
        }
    }
}

/// No report type may declare either key itself: the two are the wrapper's.
#[test]
fn no_report_type_declares_a_next_action_field() {
    fn scan(dir: &Path, hits: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                scan(&path, hits);
            } else if path.extension().is_some_and(|e| e == "rs")
                && path.file_name().is_some_and(|n| n != "main.rs")
            {
                for (n, line) in std::fs::read_to_string(&path).unwrap().lines().enumerate() {
                    let t = line.trim_start().trim_start_matches("pub ");
                    if t.starts_with("next_action:") || t.starts_with("next_action_reason:") {
                        hits.push(format!("{}:{}", path.display(), n + 1));
                    }
                }
            }
        }
    }
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut hits = Vec::new();
    for member in ["urzua-core", "urzua-cli", "urzua-mcp", "urzua-io"] {
        scan(&crates.join(member).join("src"), &mut hits);
    }
    assert!(hits.is_empty(), "a report declares its own key: {hits:?}");
}
