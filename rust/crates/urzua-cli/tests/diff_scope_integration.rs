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
    let dir = empty_repo(name, CONFIG);
    write_adr(&dir, "0001-target", "Accepted", None);
    write_adr(&dir, "0002-clean", "Accepted", None);
    write_adr(&dir, "0003-dangling", "Accepted", Some("RFC-9999"));
    tag_base(&dir);
    dir
}

fn empty_repo(name: &str, config: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-diff-scope-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    dir
}

fn tag_base(dir: &Path) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "base"]);
    git(dir, &["tag", "base"]);
}

/// A config with every rule the dependent tests need, declared once.
const RICH_CONFIG: &str = "schema_version: 2
rules:
  header.required-fields: error
  pointer.resolution: error
\x20 pointer.target-status: {level: error, not_in: [\"Superseded\"]}\n\x20 narrative-field.stale: {level: error, terminal_statuses: [\"Superseded\"]}\n\x20 relation.supersession-reciprocity: error\n  embodiment.consistency: error\n\x20 embodiment.locator-exists: error\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n\x20   known_fields: [\"Status\", \"Implements\", \"Blocked-on\", \"Supersedes / Superseded-by\", \"Embodiment\", \"Realized-by\"]\n\x20   pointer_fields: [\"Implements\"]\n    narrative_fields: [\"Blocked-on\"]\n";

/// Writes `docs/adr/<file>` with a `Status` plus any extra header lines.
fn adr(dir: &Path, file: &str, status: &str, extra: &[(&str, &str)]) {
    let mut body = format!("# {file}\n\n> Status: {status}\n");
    for (k, v) in extra {
        body.push_str(&format!("> {k}: {v}\n"));
    }
    std::fs::write(dir.join("docs/adr").join(file), body).unwrap();
}

fn rule_files(report: &serde_json::Value, rule: &str) -> Vec<String> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == rule)
        .map(|f| f["file"].as_str().unwrap().to_string())
        .collect()
}

/// The dependency-blind case in one assertion: whatever a full sweep reports
/// for `rule` on `file`, `--base` must report too.
fn assert_full_and_diff_agree(dir: &Path, rule: &str, file: &str) {
    let (_, full) = check(dir, &[]);
    assert!(
        rule_files(&full, rule).iter().any(|f| f.ends_with(file)),
        "precondition: the full check should report {rule} on {file}: {full}"
    );
    let (code, report) = check(dir, &["--base", "base"]);
    assert!(
        rule_files(&report, rule).iter().any(|f| f.ends_with(file)),
        "--base hid {rule} on {file}, which a full check reports: {report}"
    );
    assert_eq!(code, Some(1), "{report}");
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
fn an_empty_diff_is_ok_with_nothing_examined_and_says_so() {
    let dir = fixture("nothing");
    let (code, report) = check(&dir, &["--base", "base"]);
    assert_eq!(code, Some(0), "{report}");
    assert_eq!(report["status"], "ok", "{report}");
    assert_eq!(report["files_examined"], 0, "{report}");
    assert_eq!(report["scope"]["source"], "git-diff", "{report}");
    assert_eq!(report["scope"]["base"], "base", "{report}");
    // Distinguishable from a whole-corpus run, which also examines everything.
    assert_eq!(
        report["scope"]["changed_paths"],
        serde_json::json!([]),
        "{report}"
    );
    assert_eq!(
        report["scope"]["dependents"],
        serde_json::json!([]),
        "{report}"
    );
    assert!(report["scope"]["whole_corpus_reason"].is_null(), "{report}");
    assert!(
        report["notices"]
            .to_string()
            .contains("no governed record changed"),
        "an empty diff must carry a notice: {report}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_change_to_a_non_record_file_is_an_empty_diff_not_a_whole_corpus_run() {
    let dir = fixture("non-record");
    std::fs::write(dir.join("README.md"), "hello\n").unwrap();
    commit(&dir, "readme");
    let (code, report) = check(&dir, &["--base", "base"]);
    assert_eq!(code, Some(0), "{report}");
    assert_eq!(report["status"], "ok", "{report}");
    assert_eq!(report["files_examined"], 0, "{report}");
    assert_eq!(
        report["scope"]["changed_paths"],
        serde_json::json!(["README.md"]),
        "{report}"
    );
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

#[test]
fn a_deleted_target_still_reports_the_surviving_referrer() {
    let dir = empty_repo("deleted-target", RICH_CONFIG);
    adr(&dir, "ADR-1-target.md", "Accepted", &[]);
    adr(
        &dir,
        "ADR-2-referrer.md",
        "Accepted",
        &[("Implements", "ADR-1")],
    );
    tag_base(&dir);
    git(&dir, &["rm", "-q", "docs/adr/ADR-1-target.md"]);

    assert_full_and_diff_agree(&dir, "pointer.resolution", "ADR-2-referrer.md");
    let (_, report) = check(&dir, &["--base", "base"]);
    assert_eq!(report["files_examined"], 1, "{report}");
    assert_eq!(
        report["scope"]["dependents"],
        serde_json::json!(["docs/adr/ADR-2-referrer.md"]),
        "{report}"
    );
    assert_eq!(
        report["scope"]["changed_paths"],
        serde_json::json!(["docs/adr/ADR-1-target.md"]),
        "{report}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_target_renamed_to_a_different_id_still_reports_the_referrer() {
    let dir = empty_repo("renamed-target", RICH_CONFIG);
    adr(&dir, "ADR-1-target.md", "Accepted", &[]);
    adr(
        &dir,
        "ADR-2-referrer.md",
        "Accepted",
        &[("Implements", "ADR-1")],
    );
    tag_base(&dir);
    git(
        &dir,
        &["mv", "docs/adr/ADR-1-target.md", "docs/adr/ADR-5-target.md"],
    );
    commit(&dir, "renumber");

    assert_full_and_diff_agree(&dir, "pointer.resolution", "ADR-2-referrer.md");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_target_status_change_reports_the_unchanged_referrers() {
    let dir = empty_repo("status-change", RICH_CONFIG);
    adr(&dir, "ADR-1-target.md", "Accepted", &[]);
    adr(
        &dir,
        "ADR-2-implementer.md",
        "Accepted",
        &[("Implements", "ADR-1")],
    );
    adr(
        &dir,
        "ADR-3-blocked.md",
        "Accepted",
        &[("Blocked-on", "ADR-1")],
    );
    tag_base(&dir);
    adr(&dir, "ADR-1-target.md", "Superseded", &[]);
    commit(&dir, "supersede target");

    assert_full_and_diff_agree(&dir, "pointer.target-status", "ADR-2-implementer.md");
    assert_full_and_diff_agree(&dir, "narrative-field.stale", "ADR-3-blocked.md");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn supersession_reciprocity_is_reported_on_the_unchanged_peer() {
    let dir = empty_repo("reciprocity", RICH_CONFIG);
    adr(
        &dir,
        "ADR-1-old.md",
        "Accepted",
        &[("Supersedes / Superseded-by", "ADR-2")],
    );
    adr(
        &dir,
        "ADR-2-new.md",
        "Accepted",
        &[("Supersedes / Superseded-by", "ADR-1")],
    );
    tag_base(&dir);
    adr(
        &dir,
        "ADR-2-new.md",
        "Accepted",
        &[("Supersedes / Superseded-by", "\u{2014}")],
    );
    commit(&dir, "drop the back reference");

    assert_full_and_diff_agree(&dir, "relation.supersession-reciprocity", "ADR-1-old.md");
    std::fs::remove_dir_all(&dir).ok();
}

/// A repo whose `ADR-1` realizes `src/lib.rs`; the base commit holds both.
fn evidence_fixture(name: &str) -> PathBuf {
    let dir = empty_repo(name, RICH_CONFIG);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/lib.rs"), "fn a() {}\n").unwrap();
    adr(
        &dir,
        "ADR-1-realized.md",
        "Accepted",
        &[
            ("Embodiment", "Implemented"),
            ("Realized-by", "code:src/lib.rs"),
        ],
    );
    adr(&dir, "ADR-2-other.md", "Accepted", &[]);
    tag_base(&dir);
    dir
}

#[test]
fn an_evidence_only_change_reports_drift_on_the_unchanged_record() {
    let dir = evidence_fixture("evidence-drift");
    std::fs::write(dir.join("src/lib.rs"), "fn a() { println!(\"x\"); }\n").unwrap();
    commit(&dir, "touch the evidence only");

    assert_full_and_diff_agree(&dir, "embodiment.consistency", "ADR-1-realized.md");
    let (_, report) = check(&dir, &["--base", "base"]);
    assert_eq!(
        report["files_examined"], 1,
        "ADR-2 is not a dependent: {report}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn deleted_evidence_reports_the_unchanged_record() {
    let dir = evidence_fixture("evidence-deleted");
    git(&dir, &["rm", "-q", "src/lib.rs"]);

    assert_full_and_diff_agree(&dir, "embodiment.locator-exists", "ADR-1-realized.md");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_config_change_puts_the_whole_corpus_in_scope() {
    let dir = fixture("config-change");
    let config = dir.join(".urzua/config.yaml");
    let mut text = std::fs::read_to_string(&config).unwrap();
    text.push_str("\n# touched\n");
    std::fs::write(&config, text).unwrap();
    commit(&dir, "touch the config");

    let (code, report) = check(&dir, &["--base", "base"]);
    assert_eq!(report["files_examined"], 3, "{report}");
    assert_eq!(
        code,
        Some(1),
        "ADR-3's dangling pointer is in scope: {report}"
    );
    assert!(
        finding_files(&report).iter().any(|f| f.contains("ADR-3")),
        "{report}"
    );
    assert!(
        report["scope"]["whole_corpus_reason"]
            .as_str()
            .is_some_and(|r| r.contains("config")),
        "scope must say why: {report}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_newly_added_record_is_examined() {
    let dir = fixture("added");
    write_adr(&dir, "0004-new", "Accepted", Some("RFC-9998"));
    commit(&dir, "add 0004");

    let (code, report) = check(&dir, &["--base", "base"]);
    assert_eq!(code, Some(1), "{report}");
    assert_eq!(report["files_examined"], 1, "{report}");
    assert!(
        finding_files(&report).iter().all(|f| f.contains("ADR-4")),
        "{report}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_changed_path_with_spaces_is_examined() {
    let dir = fixture("spaces");
    std::fs::write(
        dir.join("docs/adr/ADR-4-has a space.md"),
        "# 4 — has a space\n\n> Status: Accepted\n> Implements: RFC-9998\n",
    )
    .unwrap();
    commit(&dir, "add a record with spaces");

    let (code, report) = check(&dir, &["--base", "base"]);
    assert_eq!(code, Some(1), "{report}");
    assert_eq!(report["files_examined"], 1, "{report}");
    assert!(
        finding_files(&report)
            .iter()
            .any(|f| f == "docs/adr/ADR-4-has a space.md"),
        "{report}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_option_like_ref_is_rejected_not_passed_to_git() {
    let dir = fixture("option-ref");
    let (code, report) = check(&dir, &["--base=-x"]);
    assert_eq!(code, Some(2), "{report}");
    assert!(
        report.to_string().contains("may not begin with"),
        "{report}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn unrelated_histories_say_there_is_no_merge_base() {
    let dir = fixture("no-merge-base");
    git(&dir, &["checkout", "-q", "--orphan", "island"]);
    commit(&dir, "unrelated root");

    let (code, report) = check(&dir, &["--base", "base"]);
    assert_eq!(code, Some(2), "{report}");
    assert!(report.to_string().contains("no merge base"), "{report}");
    std::fs::remove_dir_all(&dir).ok();
}
