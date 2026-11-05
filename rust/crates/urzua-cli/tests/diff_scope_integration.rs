//! MILE-123: `check --base <ref>` reports `scope.source: git-diff`, examining
//! and reporting on only the records that changed against the base, while
//! reference resolution still indexes the whole corpus (SPEC-2, Discovery).

mod common;

use std::path::Path;
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
fn fixture(name: &str) -> common::TempDir {
    let dir = empty_repo(name, CONFIG);
    write_adr(&dir, "0001-target", "Accepted", None);
    write_adr(&dir, "0002-clean", "Accepted", None);
    write_adr(&dir, "0003-dangling", "Accepted", Some("RFC-9999"));
    tag_base(&dir);
    dir
}

fn empty_repo(name: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-diff-scope-{name}"));
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
    let mut body = format!("---\nStatus: '{status}'\n");
    for (k, v) in extra {
        body.push_str(&format!("{k}: '{}'\n", v.replace('\'', "''")));
    }
    body.push_str(&format!("---\n# {file}\n"));
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
    let mut body = format!("---\nStatus: '{status}'\n");
    if let Some(target) = implements {
        body.push_str(&format!("Implements: '{target}'\n"));
    }
    body.push_str(&format!("---\n# {number} — {}\n", &stem[5..]));
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
}

#[test]
fn uncommitted_and_staged_edits_count_as_changed() {
    let dir = fixture("working-tree");
    write_adr(&dir, "0001-target", "Proposed", None);
    git(&dir, &["add", "-A"]);
    write_adr(&dir, "0002-clean", "Proposed", None);

    let (_, report) = check(&dir, &["--base", "base"]);
    assert_eq!(report["files_examined"], 2, "{report}");
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
}

#[test]
fn an_unknown_ref_could_not_run_rather_than_examining_nothing() {
    let dir = fixture("bad-ref");
    let (code, report) = check(&dir, &["--base", "no-such-ref"]);
    assert_eq!(code, Some(2), "{report}");
    assert_eq!(report["status"], "not-run", "{report}");
    assert!(
        report["error"]
            .as_str()
            .is_some_and(|e| e.contains("no-such-ref")),
        "the error names the ref: {report}"
    );
    // Nothing was examined, and nothing reads as a clean pass.
    assert!(report.get("findings").is_none(), "{report}");
}

#[test]
fn an_untracked_file_argument_cannot_combine_with_a_base() {
    let dir = fixture("argv-conflict");
    std::fs::write(dir.join("docs/scratch.md"), "# scratch\n").unwrap();
    let (code, report) = check(&dir, &["--base", "base", "docs/scratch.md"]);
    assert_eq!(code, Some(2), "{report}");
    assert!(report.to_string().contains("does not track"), "{report}");
}

#[test]
fn a_default_run_reports_a_null_base() {
    let dir = fixture("null-base");
    let (_, report) = check(&dir, &[]);
    assert_eq!(report["scope"]["source"], "tracked-sweep", "{report}");
    assert!(report["scope"].as_object().unwrap().contains_key("base"));
    assert!(report["scope"]["base"].is_null(), "{report}");
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
}

/// A repo whose `ADR-1` realizes `src/lib.rs`; the base commit holds both.
fn evidence_fixture(name: &str) -> common::TempDir {
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
}

#[test]
fn a_glob_locator_covering_a_changed_file_makes_the_record_a_dependent() {
    let dir = empty_repo("evidence-glob", RICH_CONFIG);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    adr(
        &dir,
        "ADR-1-realized.md",
        "Accepted",
        &[
            ("Embodiment", "Implemented"),
            ("Realized-by", "code:src/**/*.rs"),
        ],
    );
    adr(&dir, "ADR-2-other.md", "Accepted", &[]);
    tag_base(&dir);
    std::fs::write(dir.join("src/a.rs"), "fn a() { println!(\"x\"); }\n").unwrap();
    commit(&dir, "touch the globbed evidence only");

    let (_, report) = check(&dir, &["--base", "base"]);
    let dependents: Vec<&str> = report["scope"]["dependents"]
        .as_array()
        .unwrap_or_else(|| panic!("no dependents list: {report}"))
        .iter()
        .map(|d| d.as_str().unwrap())
        .collect();
    assert_eq!(
        dependents.len(),
        1,
        "only the glob-citing record is a dependent: {report}"
    );
    assert!(dependents[0].ends_with("ADR-1-realized.md"), "{report}");
    assert_eq!(report["files_examined"], 1, "{report}");
}

#[test]
fn deleted_evidence_reports_the_unchanged_record() {
    let dir = evidence_fixture("evidence-deleted");
    git(&dir, &["rm", "-q", "src/lib.rs"]);

    assert_full_and_diff_agree(&dir, "embodiment.locator-exists", "ADR-1-realized.md");
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
}

#[test]
fn a_changed_path_with_spaces_is_examined() {
    let dir = fixture("spaces");
    std::fs::write(
        dir.join("docs/adr/ADR-4-has a space.md"),
        "---\nStatus: Accepted\nImplements: RFC-9998\n---\n# 4 — has a space\n",
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
}

#[test]
fn unrelated_histories_say_there_is_no_merge_base() {
    let dir = fixture("no-merge-base");
    git(&dir, &["checkout", "-q", "--orphan", "island"]);
    commit(&dir, "unrelated root");

    let (code, report) = check(&dir, &["--base", "base"]);
    assert_eq!(code, Some(2), "{report}");
    assert!(report.to_string().contains("no merge base"), "{report}");
}

#[test]
fn a_shallow_clone_says_to_fetch_history_not_that_histories_are_unrelated() {
    let origin = fixture("shallow-origin");
    git(&origin, &["branch", "-M", "main"]);
    write_adr(&origin, "0004-later", "Accepted", None);
    commit(&origin, "later");
    let parent = common::TempDir::new("urzua-diff-scope-shallow-clone");
    let clone = parent.join("clone");
    let out = Command::new("git")
        .args(["clone", "-q", "--depth", "1"])
        .arg(format!("file://{}", origin.display()))
        .arg(&clone)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    git(&clone, &["config", "user.email", "test@example.com"]);
    git(&clone, &["config", "user.name", "test"]);
    // The base commit is not in the one-commit history: fetch it as a lone
    // commit so the ref resolves but nothing joins it to HEAD.
    git(&clone, &["fetch", "-q", "--depth", "1", "origin", "base"]);
    git(&clone, &["tag", "base", "FETCH_HEAD"]);

    let (code, report) = check(&clone, &["--base", "base"]);
    assert_eq!(code, Some(2), "{report}");
    let text = report.to_string();
    assert!(text.contains("shallow clone"), "{report}");
    assert!(
        text.contains("fetch full history") || text.contains("unshallow"),
        "{report}"
    );
    assert!(!text.contains("unrelated"), "{report}");
}

#[test]
fn a_shared_locator_pulls_in_the_unchanged_record_citing_it() {
    let config = RICH_CONFIG.replace(
        "  embodiment.consistency: error\n",
        "  embodiment.consistency: error\n  embodiment.locator-promotion-candidate: error\n",
    );
    let dir = empty_repo("shared-locator", &config);
    std::fs::create_dir_all(dir.join("src/a")).unwrap();
    std::fs::write(dir.join("src/a/x.rs"), "fn a() {}\n").unwrap();
    adr(
        &dir,
        "ADR-1-first.md",
        "Accepted",
        &[
            ("Embodiment", "Implemented"),
            ("Realized-by", "code: src/a/x.rs"),
        ],
    );
    tag_base(&dir);
    adr(
        &dir,
        "ADR-2-second.md",
        "Accepted",
        &[
            ("Embodiment", "Implemented"),
            ("Realized-by", "code: ./src/a/x.rs"),
        ],
    );
    commit(&dir, "add a second citer");

    assert_full_and_diff_agree(
        &dir,
        "embodiment.locator-promotion-candidate",
        "ADR-1-first.md",
    );
    let (_, report) = check(&dir, &["--base", "base"]);
    assert_eq!(
        report["scope"]["dependents"],
        serde_json::json!(["docs/adr/ADR-1-first.md"]),
        "{report}"
    );
}

#[test]
fn a_path_argument_that_excludes_the_changed_records_says_so() {
    let dir = evidence_fixture("path-excludes-change");
    git(&dir, &["rm", "-q", "docs/adr/ADR-2-other.md"]);
    adr(
        &dir,
        "ADR-3-referrer.md",
        "Accepted",
        &[("Implements", "ADR-2")],
    );
    // The referrer is new and staged, hence changed; ADR-2 is gone.
    git(&dir, &["add", "-A"]);
    let (code, report) = check(&dir, &["--base", "base", "src"]);
    let notices = report["notices"].to_string();
    assert!(
        !notices.contains("no governed record changed"),
        "a record changed; the notice must not deny it: {report}"
    );
    assert!(
        notices.contains("path argument"),
        "the notice should name the path argument: {report}"
    );
    assert_eq!(code, Some(0), "{report}");
}

const BARE_CONFIG: &str = "schema_version: 2\nrules: {}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n\x20   known_fields: [\"Status\"]\n    pointer_fields: []\n    narrative_fields: []\n";

#[test]
fn an_empty_diff_with_no_rule_enabled_is_not_run_like_a_full_run() {
    let dir = empty_repo("no-rules", BARE_CONFIG);
    adr(&dir, "ADR-1-a.md", "Accepted", &[]);
    tag_base(&dir);

    let (full_code, full) = check(&dir, &[]);
    assert_eq!(full["status"], "not-run", "{full}");
    assert_eq!(full_code, Some(2), "{full}");
    let (code, report) = check(&dir, &["--base", "base"]);
    assert_eq!(report["status"], "not-run", "{report}");
    assert_eq!(code, Some(2), "{report}");
}

const WAIVER_CONFIG: &str = "schema_version: 2
rules: {pointer.resolution: error}

record_types:
  adr:
    dir: \"docs/adr\"
    required_fields: []
\x20   known_fields: [\"Status\", \"Implements\"]
    pointer_fields: [\"Implements\"]
    narrative_fields: []
  waiver:
    dir: \"docs/waiver\"
    required_fields: []
    pointer_fields: []
    narrative_fields: []
";

fn waiver_repo(name: &str) -> common::TempDir {
    let dir = empty_repo(name, WAIVER_CONFIG);
    std::fs::create_dir_all(dir.join("docs/waiver")).unwrap();
    adr(
        &dir,
        "ADR-1-dangling.md",
        "Accepted",
        &[("Implements", "RFC-9999")],
    );
    write_waiver(&dir, "docs/adr/ADR-1-dangling.md");
    tag_base(&dir);
    dir
}

fn write_waiver(dir: &Path, scope: &str) {
    std::fs::write(
        dir.join("docs/waiver/0001-w.md"),
        format!(
            "---\nRule: pointer.resolution\nScope: '{scope}'\nReason: tracked elsewhere\n---\n# 0001 — Waive it\n"
        ),
    )
    .unwrap();
}

#[test]
fn a_deleted_waiver_reports_the_finding_it_stopped_waiving() {
    let dir = waiver_repo("waiver-deleted");
    git(&dir, &["rm", "-q", "docs/waiver/0001-w.md"]);
    let (_, full) = check(&dir, &[]);
    assert_eq!(full["blocking"], true, "precondition: {full}");
    let (code, report) = check(&dir, &["--base", "base"]);
    assert_eq!(code, Some(1), "{report}");
    assert!(
        finding_files(&report).iter().any(|f| f.contains("ADR-1")),
        "{report}"
    );
}

#[test]
fn a_waiver_whose_scope_changed_reports_the_finding_it_stopped_waiving() {
    let dir = waiver_repo("waiver-rescoped");
    write_waiver(&dir, "docs/adr/ADR-9-elsewhere.md");
    let (_, full) = check(&dir, &[]);
    assert_eq!(full["blocking"], true, "precondition: {full}");
    let (code, report) = check(&dir, &["--base", "base"]);
    assert_eq!(code, Some(1), "{report}");
    assert!(
        finding_files(&report).iter().any(|f| f.contains("ADR-1")),
        "{report}"
    );
}

#[test]
fn a_comma_joined_pointer_value_traces_each_reference() {
    let dir = empty_repo("comma-list", RICH_CONFIG);
    adr(&dir, "ADR-1-a.md", "Accepted", &[]);
    adr(&dir, "ADR-3-b.md", "Accepted", &[]);
    adr(
        &dir,
        "ADR-5-ref.md",
        "Accepted",
        &[("Implements", "ADR-1,ADR-3")],
    );
    tag_base(&dir);
    git(&dir, &["rm", "-q", "docs/adr/ADR-3-b.md"]);

    assert_full_and_diff_agree(&dir, "pointer.resolution", "ADR-5-ref.md");
}

#[test]
fn a_changed_file_outside_its_declared_dir_keeps_its_finding() {
    let config = RICH_CONFIG.replace(
        "rules:\n",
        "rules:\n  type.record-outside-declared-dir: warn\n",
    );
    let dir = empty_repo("outside-dir", &config);
    adr(&dir, "ADR-1-a.md", "Accepted", &[]);
    tag_base(&dir);
    std::fs::create_dir_all(dir.join("docs/adr/sub")).unwrap();
    std::fs::write(
        dir.join("docs/adr/sub/ADR-2-nested.md"),
        "---\nStatus: Accepted\n---\n# nested\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);

    let (_, full) = check(&dir, &[]);
    assert!(
        rule_files(&full, "type.record-outside-declared-dir")
            .iter()
            .any(|f| f.ends_with("ADR-2-nested.md")),
        "precondition: {full}"
    );
    let (_, report) = check(&dir, &["--base", "base"]);
    assert!(
        rule_files(&report, "type.record-outside-declared-dir")
            .iter()
            .any(|f| f.ends_with("ADR-2-nested.md")),
        "{report}"
    );
}

#[cfg(unix)]
#[test]
fn retargeting_a_symlinked_config_puts_the_whole_corpus_in_scope() {
    let dir = empty_repo("symlink-config", CONFIG);
    std::fs::remove_file(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::create_dir_all(dir.join("cfg")).unwrap();
    std::fs::write(dir.join("cfg/a.yaml"), CONFIG).unwrap();
    std::fs::write(dir.join("cfg/b.yaml"), format!("{CONFIG}\n# b\n")).unwrap();
    std::os::unix::fs::symlink("../cfg/a.yaml", dir.join(".urzua/config.yaml")).unwrap();
    write_adr(&dir, "0001-target", "Accepted", None);
    write_adr(&dir, "0003-dangling", "Accepted", Some("RFC-9999"));
    tag_base(&dir);
    std::fs::remove_file(dir.join(".urzua/config.yaml")).unwrap();
    std::os::unix::fs::symlink("../cfg/b.yaml", dir.join(".urzua/config.yaml")).unwrap();

    let (code, report) = check(&dir, &["--base", "base"]);
    assert!(
        report["scope"]["whole_corpus_reason"].is_string(),
        "{report}"
    );
    assert_eq!(code, Some(1), "{report}");
}

#[test]
fn an_unstaged_deletion_cannot_be_loaded_so_the_run_could_not_run() {
    // Existing behaviour (BUG-82): a tracked record missing from disk and not
    // staged for deletion is rejected by load_records, diff-scoped or not.
    let dir = empty_repo("unstaged-delete", RICH_CONFIG);
    adr(&dir, "ADR-1-target.md", "Accepted", &[]);
    adr(&dir, "ADR-2-ref.md", "Accepted", &[("Implements", "ADR-1")]);
    tag_base(&dir);
    std::fs::remove_file(dir.join("docs/adr/ADR-1-target.md")).unwrap();

    let (full_code, _) = check(&dir, &[]);
    let (code, report) = check(&dir, &["--base", "base"]);
    assert_eq!(full_code, Some(2));
    assert_eq!(code, Some(2), "{report}");
}

const TRANSITIONS_CONFIG: &str = "schema_version: 2
rules:
  status.transition-allowed: error
record_types:
  adr:
    dir: \"docs/adr\"
    required_fields: [\"Status\"]
    known_fields: [\"Status\"]
    statuses: {values: [Draft, Accepted], initial: [Draft], transitions: {Draft: [Accepted], Accepted: []}}
";

fn transition_execution(report: &serde_json::Value) -> &serde_json::Value {
    report["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "status.transition-allowed")
        .unwrap_or_else(|| panic!("rule not executed: {report}"))
}

#[test]
fn a_base_copy_that_cannot_be_read_is_unreadable_not_new() {
    let dir = empty_repo("base-unreadable", TRANSITIONS_CONFIG);
    // The base copy is Accepted but holds a byte that is not UTF-8; the
    // working copy is valid and moves it back to Draft, a disallowed edge.
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        b"---\nStatus: 'Accepted'\n---\n# x \xff\n",
    )
    .unwrap();
    tag_base(&dir);
    adr(&dir, "ADR-1-x.md", "Draft", &[]);

    let (_, report) = check(&dir, &["--base", "base"]);
    let execution = transition_execution(&report);
    assert_eq!(
        execution["population"]["unreadable"], 1,
        "a base copy that cannot be read is unreadable, not a new record: {report}"
    );
    let notices = report["notices"].as_array().unwrap();
    assert!(
        notices
            .iter()
            .any(|n| n["subject"] == "status.transition-allowed"
                && n["message"].as_str().unwrap().contains("merge base")
                && n["message"].as_str().unwrap().contains("UTF-8")),
        "the read error is reported: {report}"
    );
}

#[test]
fn a_path_the_base_lacks_is_still_a_new_record() {
    let dir = empty_repo("base-absent", TRANSITIONS_CONFIG);
    adr(&dir, "ADR-1-x.md", "Draft", &[]);
    tag_base(&dir);
    adr(&dir, "ADR-2-new.md", "Draft", &[]);
    git(&dir, &["add", "-A"]);

    let (_, report) = check(&dir, &["--base", "base"]);
    let execution = transition_execution(&report);
    assert_eq!(execution["population"]["unreadable"], 0, "{report}");
    assert_eq!(execution["population"]["examined"], 1, "{report}");
    assert!(finding_files(&report).is_empty(), "{report}");
}
