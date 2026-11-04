//! `embodiment.back-pointer-resolution` (`MILE-128`) against the real binary:
//! an `Implements:` source comment is code-tier evidence for the record it
//! cites, in `check` and in `fix`, and one citing no record is reported.

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
    Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

/// One ADR stating `Embodiment: Implemented` with only a `spec:` locator, so
/// it computes to `Specified` unless a source comment cites it.
fn repo(name: &str, back_pointer_rule: bool, source: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-backptr-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
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
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn without_the_rule_the_comment_is_not_evidence() {
    let dir = repo("off", false, "// Implements: ADR-1\n");
    let out = run_urzua(&dir, &["check"]);
    assert_eq!(findings(&out, "embodiment.consistency").len(), 1);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn without_a_comment_the_stated_tier_disagrees() {
    let dir = repo("none", true, "fn a() {}\n");
    let out = run_urzua(&dir, &["check"]);
    assert_eq!(findings(&out, "embodiment.consistency").len(), 1);
    std::fs::remove_dir_all(&dir).ok();
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
    std::fs::remove_dir_all(&cited).ok();

    let uncited = repo("fix-uncited", true, "fn a() {}\n");
    let out = run_urzua(&uncited, &["fix"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(
        parsed["repairs"].as_array().map(Vec::len),
        Some(1),
        "{parsed}"
    );
    std::fs::remove_dir_all(&uncited).ok();
}

/// `embodiment.back-pointer-decision-accepted` (`MILE-133`): a back-pointer to
/// a record that is not yet Accepted is reported, with `--base` limiting the
/// judgement to files the diff changed.
fn gate_repo(name: &str, status: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-gate-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
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
         values: [Proposed, Accepted]\n      success: [Accepted]\n",
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
