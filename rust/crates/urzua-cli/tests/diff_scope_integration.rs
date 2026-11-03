//! MILE-123: `check --base <ref>` reports `scope.source: git-diff`, examining
//! and reporting on only the records that changed against the base, while
//! reference resolution still indexes the whole corpus (SPEC-2, Discovery).

use std::path::{Path, PathBuf};
use std::process::Command;

const CONFIG: &str = "schema_version: 2\nrules: {header.required-fields: error, pointer.resolution: error}\n\n\
record_types:\n\
\x20 adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n\
\x20   known_fields: [\"Status\", \"Implements\"]\n    pointer_fields: [\"Implements\"]\n    narrative_fields: []\n";

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git should run");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A repo with one base commit holding three ADRs: 0001 is a valid target,
/// 0002 is clean, 0003 carries a dangling pointer (an error). The base is
/// tagged `base`; nothing has changed since.
fn fixture(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-diff-scope-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    write_adr(&dir, "0001-target", "Accepted", None);
    write_adr(&dir, "0002-clean", "Accepted", None);
    write_adr(&dir, "0003-dangling", "Accepted", Some("RFC-9999"));
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    git(&dir, &["tag", "base"]);
    dir
}

fn write_adr(dir: &Path, stem: &str, status: &str, implements: Option<&str>) {
    let number = &stem[..4];
    let mut body = format!("# {number} — {}\n\n> Status: {status}\n", &stem[5..]);
    if let Some(target) = implements {
        body.push_str(&format!("> Implements: {target}\n"));
    }
    std::fs::write(
        dir.join(format!(
            "docs/adr/ADR-{}-{}.md",
            number.trim_start_matches('0'),
            &stem[5..]
        )),
        body,
    )
    .unwrap();
}

fn commit(dir: &Path, message: &str) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", message]);
}

fn check(dir: &Path, args: &[&str]) -> (Option<i32>, serde_json::Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .arg("check")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua should run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n{stdout}"));
    (out.status.code(), json)
}

fn finding_files(report: &serde_json::Value) -> Vec<String> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["file"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn only_the_changed_record_is_examined_and_the_base_is_reported() {
    let dir = fixture("changed");
    write_adr(&dir, "0002-clean", "Proposed", None);
    commit(&dir, "touch 0002");

    let (_, report) = check(&dir, &["--base", "base"]);
    assert_eq!(report["scope"]["source"], "git-diff", "{report}");
    assert_eq!(report["scope"]["base"], "base", "{report}");
    assert_eq!(report["files_examined"], 1, "{report}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_error_in_an_unchanged_record_is_not_reported() {
    let dir = fixture("unchanged-error");
    write_adr(&dir, "0002-clean", "Proposed", None);
    commit(&dir, "touch 0002");

    // The full sweep sees 0003's dangling pointer; the diff run must not.
    let (full_code, full) = check(&dir, &[]);
    assert_eq!(full_code, Some(1), "{full}");
    assert!(finding_files(&full).iter().any(|f| f.contains("ADR-3")));

    let (code, report) = check(&dir, &["--base", "base"]);
    assert!(finding_files(&report).is_empty(), "{report}");
    assert_eq!(code, Some(0), "{report}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_changed_record_is_still_judged_and_blocks() {
    let dir = fixture("changed-error");
    write_adr(&dir, "0003-dangling", "Proposed", Some("RFC-9999"));
    commit(&dir, "touch 0003");

    let (code, report) = check(&dir, &["--base", "base"]);
    assert_eq!(code, Some(1), "{report}");
    assert_eq!(report["files_examined"], 1, "{report}");
    assert!(!finding_files(&report).is_empty(), "{report}");
    assert!(finding_files(&report).iter().all(|f| f.contains("ADR-3")));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_pointer_to_an_unchanged_record_still_resolves() {
    let dir = fixture("resolves");
    // 0002 now points at 0001, which did not change: indexing only the diff
    // would report the target missing.
    write_adr(&dir, "0002-clean", "Accepted", Some("ADR-1"));
    commit(&dir, "point 0002 at 0001");

    let (code, report) = check(&dir, &["--base", "base"]);
    assert_eq!(report["files_examined"], 1, "{report}");
    assert!(
        finding_files(&report).is_empty(),
        "an unchanged target must not be reported missing: {report}"
    );
    assert_eq!(code, Some(0), "{report}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn uncommitted_and_staged_edits_count_as_changed() {
    let dir = fixture("working-tree");
    write_adr(&dir, "0001-target", "Proposed", None);
    git(&dir, &["add", "-A"]);
    write_adr(&dir, "0002-clean", "Proposed", None);

    let (_, report) = check(&dir, &["--base", "base"]);
    assert_eq!(report["files_examined"], 2, "{report}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn nothing_changed_is_reported_as_not_run_with_the_base_named() {
    let dir = fixture("nothing");
    let (code, report) = check(&dir, &["--base", "base"]);
    assert_eq!(report["status"], "not-run", "{report}");
    assert_eq!(report["files_examined"], 0, "{report}");
    assert_eq!(report["scope"]["source"], "git-diff", "{report}");
    assert_eq!(report["scope"]["base"], "base", "{report}");
    assert_ne!(code, Some(1), "{report}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_comparison_starts_at_the_merge_base_not_the_moving_tip() {
    let dir = fixture("merge-base");
    // `trunk` moves on after the feature branch forks, changing 0001 only.
    git(&dir, &["checkout", "-q", "-b", "trunk"]);
    write_adr(&dir, "0001-target", "Proposed", None);
    commit(&dir, "trunk moved on");
    git(&dir, &["tag", "tip"]);
    git(&dir, &["checkout", "-q", "-b", "feature", "base"]);
    write_adr(&dir, "0002-clean", "Proposed", None);
    commit(&dir, "feature change");

    // Against the tip itself, 0001 differs only because trunk moved; the
    // feature branch changed 0002 alone.
    let (_, report) = check(&dir, &["--base", "tip"]);
    assert_eq!(report["files_examined"], 1, "{report}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_path_argument_narrows_the_diff_further() {
    let dir = fixture("path-narrow");
    write_adr(&dir, "0001-target", "Proposed", None);
    write_adr(&dir, "0002-clean", "Proposed", None);
    commit(&dir, "touch two");

    let (_, report) = check(&dir, &["--base", "base", "docs/adr/ADR-1-target.md"]);
    assert_eq!(report["files_examined"], 1, "{report}");
    assert_eq!(report["scope"]["source"], "git-diff", "{report}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_unknown_ref_could_not_run_rather_than_examining_nothing() {
    let dir = fixture("bad-ref");
    let (code, report) = check(&dir, &["--base", "no-such-ref"]);
    assert_eq!(code, Some(2), "{report}");
    assert!(report.to_string().contains("no-such-ref"), "{report}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_untracked_file_argument_cannot_combine_with_a_base() {
    let dir = fixture("argv-conflict");
    std::fs::write(dir.join("docs/scratch.md"), "# scratch\n").unwrap();
    let (code, report) = check(&dir, &["--base", "base", "docs/scratch.md"]);
    assert_eq!(code, Some(2), "{report}");
    assert!(report.to_string().contains("does not track"), "{report}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_default_run_reports_a_null_base() {
    let dir = fixture("null-base");
    let (_, report) = check(&dir, &[]);
    assert_eq!(report["scope"]["source"], "tracked-sweep", "{report}");
    assert!(report["scope"].as_object().unwrap().contains_key("base"));
    assert!(report["scope"]["base"].is_null(), "{report}");
    std::fs::remove_dir_all(&dir).ok();
}
