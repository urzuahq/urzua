//! `embodiment.back-pointer-resolution` (`MILE-128`) against the real binary:
//! an `Implements:` source comment is code-tier evidence for the record it
//! cites, in `check` and in `fix`, and one citing no record is reported.
//!
//! Verifies: ADR-66

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

/// One ADR stating `Embodiment: Implemented` with only a `spec:` locator, so
/// it computes to `Specified` unless a source comment cites it.
fn repo(name: &str, back_pointer_rule: bool, source: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-backptr-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("docs/spec")).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    let rule = if back_pointer_rule {
        "  embodiment.back-pointer-resolution:\n    level: error\n    back_pointer_paths: [\"src\"]\n"
    } else {
        ""
    };
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules:\n  embodiment.consistency: error\n{rule}\
             record_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    \
             required_fields: [\"Status\", \"Embodiment\", \"Realized-by\"]\n    \
             header_shape: \"yaml-frontmatter\"\n"
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\nEmbodiment: Implemented\nRealized-by: spec:docs/spec/s.md\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(dir.join("docs/spec/s.md"), "spec\n").unwrap();
    std::fs::write(dir.join("src/lib.rs"), source).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn findings(out: &std::process::Output, rule: &str) -> Vec<serde_json::Value> {
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == rule)
        .cloned()
        .collect()
}

#[test]
fn a_cited_record_computes_to_implemented_and_a_dangling_citation_is_reported() {
    let dir = repo("check", true, "// Implements: ADR-1, ADR-9\nfn a() {}\n");
    let out = run_urzua(&dir, &["check"]);
    assert!(findings(&out, "embodiment.consistency").is_empty());
    let dangling = findings(&out, "embodiment.back-pointer-resolution");
    assert_eq!(
        dangling.len(),
        1,
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(dangling[0]["file"], "src/lib.rs");
    assert_eq!(dangling[0]["line"], 1);
    assert!(dangling[0]["message"].as_str().unwrap().contains("ADR-9"));
}

#[test]
fn without_the_rule_the_comment_is_not_evidence() {
    let dir = repo("off", false, "// Implements: ADR-1\n");
    let out = run_urzua(&dir, &["check"]);
    assert_eq!(findings(&out, "embodiment.consistency").len(), 1);
}

#[test]
fn without_a_comment_the_stated_tier_disagrees() {
    let dir = repo("none", true, "fn a() {}\n");
    let out = run_urzua(&dir, &["check"]);
    assert_eq!(findings(&out, "embodiment.consistency").len(), 1);
}

#[test]
fn fix_detect_agrees_with_check_about_a_cited_record() {
    let cited = repo("fix-cited", true, "// Implements: ADR-1\n");
    let out = run_urzua(&cited, &["fix"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(
        parsed["repairs"].as_array().map(Vec::len),
        Some(0),
        "{parsed}"
    );

    let uncited = repo("fix-uncited", true, "fn a() {}\n");
    let out = run_urzua(&uncited, &["fix"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(
        parsed["repairs"].as_array().map(Vec::len),
        Some(1),
        "{parsed}"
    );
}

/// `embodiment.back-pointer-decision-accepted` (`MILE-133`): a back-pointer to
/// a record that is not yet Accepted is reported, with `--base` limiting the
/// judgement to files the diff changed.
fn gate_repo(name: &str, status: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-gate-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n  embodiment.back-pointer-decision-accepted:\n    level: error\n    \
         back_pointer_paths: [\"src\"]\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    \
         required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n    statuses:\n      \
         values: [Proposed, Accepted]\n      success: [Accepted]\n      implementable: [Accepted]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        format!("---\nStatus: {status}\n---\n# 1 — X\n"),
    )
    .unwrap();
    std::fs::write(dir.join("src/old.rs"), "// Implements: ADR-1\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

#[test]
fn a_back_pointer_to_a_proposed_decision_is_reported_and_an_accepted_one_is_not() {
    let proposed = gate_repo("proposed", "Proposed");
    let out = run_urzua(&proposed, &["check"]);
    let found = findings(&out, "embodiment.back-pointer-decision-accepted");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0]["file"], "src/old.rs");
    assert_eq!(out.status.code(), Some(1));

    let accepted = gate_repo("accepted", "Accepted");
    let out = run_urzua(&accepted, &["check"]);
    assert!(findings(&out, "embodiment.back-pointer-decision-accepted").is_empty());
}

#[test]
fn on_a_base_run_only_changed_source_files_are_judged() {
    let dir = gate_repo("diff", "Proposed");
    git(&dir, &["checkout", "-q", "-b", "work"]);
    std::fs::write(dir.join("src/new.rs"), "// Implements: ADR-1\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "change"]);
    let out = run_urzua(&dir, &["check", "--base", "main"]);
    let found = findings(&out, "embodiment.back-pointer-decision-accepted");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0]["file"], "src/new.rs");
}

/// The suggested edit for an uncovered path lands in the configuration, not in
/// the declared path the finding is about (`ADR-73`).
#[test]
fn an_uncovered_path_action_targets_the_config_file() {
    let dir = repo("uncovered-target", true, "// Implements: ADR-1\n");
    let config = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        config.replace(
            "back_pointer_paths: [\"src\"]",
            "back_pointer_paths: [\"src\", \"gone\"]\n    test_paths: [\"src/gone-tests\"]",
        ),
    )
    .unwrap();
    git(&dir, &["commit", "-qam", "uncovered"]);
    let out = run_urzua(&dir, &["check"]);
    let found = findings(&out, "embodiment.back-pointer-resolution");
    for (path, key) in [
        ("gone", "back_pointer_paths"),
        ("src/gone-tests", "test_paths"),
    ] {
        let f = found
            .iter()
            .find(|f| f["file"] == path)
            .unwrap_or_else(|| panic!("no finding for {path}: {found:?}"));
        let action = &f["suggested_action"];
        assert_eq!(action["kind"], "edit-config", "{f}");
        assert_eq!(action["target"]["file"], ".urzua/config.yaml", "{f}");
        assert_eq!(
            action["target"]["key_path"],
            serde_json::json!(["rules", "embodiment.back-pointer-resolution", key]),
            "{f}"
        );
    }
}

#[test]
fn a_decision_uncovered_path_action_targets_the_config_file() {
    let dir = gate_repo("uncovered-target", "Accepted");
    let config = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        config.replace(
            "back_pointer_paths: [\"src\"]",
            "back_pointer_paths: [\"src\", \"gone\"]",
        ),
    )
    .unwrap();
    git(&dir, &["commit", "-qam", "uncovered"]);
    let out = run_urzua(&dir, &["check"]);
    let found = findings(&out, "embodiment.back-pointer-decision-accepted");
    let f = found
        .iter()
        .find(|f| f["file"] == "gone")
        .unwrap_or_else(|| panic!("no finding for gone: {found:?}"));
    assert_eq!(
        f["suggested_action"]["target"]["file"], ".urzua/config.yaml",
        "{f}"
    );
}

// ---- The declared per-line ignore marker (`ADR-80`, amended 2026-11-08) ----

/// A repository with both back-pointer rules, `embodiment.consistency`, ADR-1
/// (Accepted, stating `Implemented` with only a `spec:` locator) and ADR-2
/// (Proposed). `marker` is the declared `ignore_marker`, or none.
fn marker_repo(name: &str, marker: Option<&str>, source: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-marker-{name}"));
    for d in ["docs/adr", "docs/spec", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    let ignore = marker
        .map(|m| format!("    ignore_marker: \"{m}\"\n"))
        .unwrap_or_default();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules:\n  embodiment.consistency: error\n  \
             embodiment.back-pointer-resolution:\n    level: error\n    back_pointer_paths: [\"src\"]\n{ignore}  \
             embodiment.back-pointer-decision-accepted:\n    level: error\n    back_pointer_paths: [\"src\"]\n{ignore}\
             record_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    \
             required_fields: [\"Status\", \"Embodiment\", \"Realized-by\"]\n    \
             header_shape: \"yaml-frontmatter\"\n    statuses:\n      \
             values: [Proposed, Accepted]\n      success: [Accepted]\n      implementable: [Accepted]\n"
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\nEmbodiment: Implemented\nRealized-by: spec:docs/spec/s.md\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-2-y.md"),
        "---\nStatus: Proposed\nEmbodiment: Not started\nRealized-by: null\n---\n# 2 — Y\n",
    )
    .unwrap();
    std::fs::write(dir.join("docs/spec/s.md"), "spec\n").unwrap();
    std::fs::write(dir.join("src/lib.rs"), source).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn report(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).expect("json")
}

/// A fixture string literal whose line reads as a back-pointer to a record
/// that does not exist; the first line is a real citation of ADR-1.
const LITERAL: &str =
    "// Implements: ADR-1\nconst S: &str = \"\n// Implements: ADR-9999 MARK\n\";\n";

#[test]
fn a_declared_marker_excuses_the_line_and_a_real_citation_elsewhere_still_resolves() {
    let dir = marker_repo(
        "excused",
        Some("urzua:ignore"),
        &LITERAL.replace("MARK", "urzua:ignore"),
    );
    let out = run_urzua(&dir, &["check"]);
    let v = report(&out);
    assert!(
        findings(&out, "embodiment.back-pointer-resolution").is_empty(),
        "{v}"
    );
    // The citation of ADR-1 on another line of the same file still counts.
    assert!(findings(&out, "embodiment.consistency").is_empty(), "{v}");
    assert_eq!(out.status.code(), Some(0), "{v}");
}

#[test]
fn the_count_of_excused_lines_is_disclosed() {
    let dir = marker_repo(
        "disclosed",
        Some("urzua:ignore"),
        &LITERAL.replace("MARK", "urzua:ignore"),
    );
    let out = run_urzua(&dir, &["check"]);
    let v = report(&out);
    let text = v.to_string();
    for rule in [
        "embodiment.back-pointer-resolution",
        "embodiment.back-pointer-decision-accepted",
    ] {
        let exec = v["rules_executed"]
            .as_array()
            .and_then(|r| r.iter().find(|e| e["rule"] == rule))
            .unwrap_or_else(|| panic!("no execution for {rule}: {text}"));
        assert_eq!(exec["population"]["excused_lines"], 1, "{exec}");
    }
    assert!(
        v["notices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["message"].as_str().unwrap_or("").contains("1 line")),
        "{text}"
    );
}

#[test]
fn an_undeclared_marker_changes_nothing() {
    // The same text with no `ignore_marker` declared: the line is reported, and
    // neither the counter nor a notice appears.
    let dir = marker_repo("undeclared", None, &LITERAL.replace("MARK", "urzua:ignore"));
    let out = run_urzua(&dir, &["check"]);
    let v = report(&out);
    let dangling = findings(&out, "embodiment.back-pointer-resolution");
    assert_eq!(dangling.len(), 1, "{v}");
    assert_eq!(dangling[0]["line"], 3);
    assert!(!v.to_string().contains("excused_lines"), "{v}");
    assert!(!v.to_string().contains("ignore marker"), "{v}");
}

#[test]
fn an_excused_line_is_not_evidence_in_check_or_fix() {
    let source = "// Implements: ADR-1 urzua:ignore\n";
    let dir = marker_repo("not-evidence", Some("urzua:ignore"), source);
    let out = run_urzua(&dir, &["check"]);
    assert_eq!(
        findings(&out, "embodiment.consistency").len(),
        1,
        "{}",
        report(&out)
    );
    let fixed = report(&run_urzua(&dir, &["fix"]));
    assert_eq!(
        fixed["repairs"].as_array().map(Vec::len),
        Some(1),
        "{fixed}"
    );
    // Without the marker declared the same line is evidence.
    let undeclared = marker_repo("evidence", None, source);
    let out = run_urzua(&undeclared, &["check"]);
    assert!(findings(&out, "embodiment.consistency").is_empty());
}

#[test]
fn the_decision_rule_honours_the_marker_too() {
    let source = "// Implements: ADR-2 urzua:ignore\n// Implements: ADR-1\n";
    let dir = marker_repo("decision", Some("urzua:ignore"), source);
    let out = run_urzua(&dir, &["check"]);
    assert!(
        findings(&out, "embodiment.back-pointer-decision-accepted").is_empty(),
        "{}",
        report(&out)
    );
    let undeclared = marker_repo("decision-off", None, source);
    let out = run_urzua(&undeclared, &["check"]);
    assert_eq!(
        findings(&out, "embodiment.back-pointer-decision-accepted").len(),
        1
    );
}

#[test]
fn a_marker_on_a_line_with_no_back_pointer_is_reported() {
    let dir = marker_repo(
        "unused",
        Some("urzua:ignore"),
        "// Implements: ADR-1\nfn a() {} // urzua:ignore\n",
    );
    let out = run_urzua(&dir, &["check"]);
    let found = findings(&out, "embodiment.back-pointer-resolution");
    assert_eq!(found.len(), 1, "{}", report(&out));
    assert_eq!(found[0]["line"], 2);
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("suppresses nothing"));
}
