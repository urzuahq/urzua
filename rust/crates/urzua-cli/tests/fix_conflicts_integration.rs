//! MILE-284: `urzua fix --conflicts` resolves a record that two branches each
//! re-pinned, and refuses one whose real content or pinned file conflicts.
//! Real git merges and rebases in a temp repo, real binary; output is JSON.

mod common;

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(["-c", "merge.conflictStyle=merge", "-c", "core.editor=true"])
        .args(args)
        .current_dir(dir)
        .env("GIT_EDITOR", "true")
        .output()
        .expect("git should run")
        .status
        .success()
}

fn ok(dir: &Path, args: &[&str]) {
    assert!(git(dir, args), "git {args:?} failed");
}

fn commit_all(dir: &Path, msg: &str) {
    ok(dir, &["add", "-A"]);
    ok(dir, &["commit", "-q", "-m", msg]);
}

fn urzua(dir: &Path, args: &[&str]) -> (Option<i32>, serde_json::Value) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua should run");
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"));
    (out.status.code(), v)
}

fn sha256(content: &str) -> String {
    use std::io::Write;
    let mut child = Command::new("sha256sum")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(content.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    String::from_utf8(out.stdout).unwrap()[..64].to_string()
}

const CONFIG: &str = "schema_version: 2\nrules: {embodiment.consistency: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    revision_log: {marker: \"**Revision log**\", version_bump: {field: Version, classes: [substantive]}}\n";
const A_RS: &str = "fn f() {\n    1;\n}\n\nfn spacer() {}\nfn spacer2() {}\nfn spacer3() {}\n\nfn g() {\n    1;\n}\n";
const REC: &str = "docs/adr/ADR-1-a.md";

fn record(hash: &str, version: &str, body: &str) -> String {
    format!(
        "---\nVersion: '{version}'\nEmbodiment: Implemented\nRealized-by: 'spec:docs/s.md, code:src/a.rs@sha256:{hash}, test:t.rs'\n---\n# ADR-1: T\n\n{body}\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
    )
}

/// Re-pins the record to `src/a.rs` as it is, at `version`, with one row
/// dated `date`; `body` replaces the body line when given.
fn repin_as(dir: &Path, date: &str, version: &str, who: &str, body: Option<&str>, class: &str) {
    let text = std::fs::read_to_string(dir.join(REC)).unwrap();
    let old = text.split("src/a.rs@sha256:").nth(1).unwrap()[..64].to_string();
    let new = sha256(&std::fs::read_to_string(dir.join("src/a.rs")).unwrap());
    let mut text = text
        .replace(&old, &new)
        .replace("Version: '1'", &format!("Version: '{version}'"));
    if let Some(b) = body {
        text = text.replace("Body.", b);
    }
    text.push_str(&format!(
        "> | {date} | Re-baselined by {who}: re-pinned `src/a.rs`. | **{class}** |\n"
    ));
    std::fs::write(dir.join(REC), text).unwrap();
}

/// main holds the record pinning `src/a.rs`; branch x edits `f`, branch y
/// edits `g`, each re-pins (y also edits `b_rs_y` into `src/b.rs` when given).
fn fixture(name: &str, x_body: Option<&str>, y_body: Option<&str>) -> common::TempDir {
    fixture_with(name, CONFIG, ("2", "3", "structural"), (x_body, y_body))
}

fn fixture_with(
    name: &str,
    config: &str,
    (x_version, y_version, class): (&str, &str, &str),
    (x_body, y_body): (Option<&str>, Option<&str>),
) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-conflicts-{name}"));
    for d in ["docs/adr", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    ok(&dir, &["init", "-q", "-b", "main"]);
    ok(&dir, &["config", "user.email", "test@example.com"]);
    ok(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join("docs/s.md"), "s\n").unwrap();
    std::fs::write(dir.join("t.rs"), "t\n").unwrap();
    std::fs::write(dir.join("src/a.rs"), A_RS).unwrap();
    std::fs::write(dir.join(REC), record(&sha256(A_RS), "1", "Body.")).unwrap();
    commit_all(&dir, "base");
    ok(&dir, &["checkout", "-q", "-b", "x"]);
    std::fs::write(dir.join("src/a.rs"), A_RS.replacen("    1;", "    2;", 1)).unwrap();
    repin_as(&dir, "2026-03-02", x_version, "x", x_body, class);
    commit_all(&dir, "x");
    ok(&dir, &["checkout", "-q", "main"]);
    ok(&dir, &["checkout", "-q", "-b", "y"]);
    let g_edited = {
        let at = A_RS.rfind("    1;").unwrap();
        format!("{}    3;{}", &A_RS[..at], &A_RS[at + 6..])
    };
    std::fs::write(dir.join("src/a.rs"), g_edited).unwrap();
    repin_as(&dir, "2026-03-01", y_version, "y", y_body, class);
    commit_all(&dir, "y");
    ok(&dir, &["checkout", "-q", "main"]);
    ok(&dir, &["merge", "-q", "--ff-only", "x"]);
    ok(&dir, &["checkout", "-q", "y"]);
    dir
}

fn apply(dir: &Path) -> (Option<i32>, serde_json::Value) {
    urzua(
        dir,
        &[
            "fix",
            "--conflicts",
            "--apply",
            "--force",
            "--by",
            "tester",
            "--actor",
            "agent",
        ],
    )
}

fn assert_resolved(dir: &Path) {
    let text = std::fs::read_to_string(dir.join(REC)).unwrap();
    let merged = sha256(&std::fs::read_to_string(dir.join("src/a.rs")).unwrap());
    assert!(!text.contains("<<<<<<<"), "{text}");
    assert!(
        text.contains(&format!("src/a.rs@sha256:{merged}")),
        "{text}"
    );
    assert!(text.contains("Version: '3'"), "{text}");
    let y = text.find("Re-baselined by y").expect("y's row kept");
    let x = text.find("Re-baselined by x").expect("x's row kept");
    let m = text.find("Pin recomputed on merge").expect("the merge row");
    assert!(y < x && x < m, "rows in date order, merge row last: {text}");
    assert!(text.contains("No person wrote a reason"), "{text}");
    assert!(text.contains("tester"), "{text}");
}

fn assert_clean_after_commit(dir: &Path, continue_args: &[&str]) {
    ok(dir, &["add", REC]);
    ok(dir, continue_args);
    let (code, v) = urzua(dir, &["check"]);
    assert_eq!(code, Some(0), "{v}");
    let (code, v) = urzua(dir, &["check", "--base", "main"]);
    assert_eq!(code, Some(0), "{v}");
    let (code, v) = urzua(dir, &["fix", "--rebaseline"]);
    assert_eq!((code, v["status"].as_str()), (Some(0), Some("ok")), "{v}");
}

#[test]
fn a_record_both_merged_sides_re_pinned_is_resolved_on_merge_observed_failing() {
    let dir = fixture("merge", None, None);
    assert!(!git(&dir, &["merge", "-q", "main"]), "the merge conflicts");
    let before = std::fs::read_to_string(dir.join(REC)).unwrap();
    for cmd in [&["check"][..], &["fix", "--rebaseline"][..]] {
        let (_, v) = urzua(&dir, cmd);
        assert_eq!(v["next_action"], "urzua fix --conflicts", "{cmd:?}: {v}");
    }
    let (code, v) = urzua(&dir, &["fix", "--conflicts"]);
    assert_eq!(
        (code, v["status"].as_str()),
        (Some(0), Some("resolvable")),
        "{v}"
    );
    assert_eq!(v["conflicts"][0]["kind"], "pin", "{v}");
    assert_eq!(v["conflicts"][0]["record"], REC, "{v}");
    assert_eq!(std::fs::read_to_string(dir.join(REC)).unwrap(), before);
    let (code, v) = apply(&dir);
    assert_eq!(
        (code, v["status"].as_str()),
        (Some(0), Some("resolved")),
        "{v}"
    );
    assert_eq!(v["conflicts"][0]["pins"][0]["recomputed"], true, "{v}");
    assert!(
        v["next_action"].as_str().unwrap().starts_with("git add"),
        "{v}"
    );
    assert_resolved(&dir);
    assert_clean_after_commit(&dir, &["commit", "-q", "--no-edit"]);
}

#[test]
fn a_record_both_rebased_sides_re_pinned_keeps_both_rows_in_date_order() {
    let dir = fixture("rebase", None, None);
    assert!(
        !git(&dir, &["rebase", "-q", "main"]),
        "the rebase conflicts"
    );
    let (code, v) = apply(&dir);
    assert_eq!(
        (code, v["status"].as_str()),
        (Some(0), Some("resolved")),
        "{v}"
    );
    assert_resolved(&dir);
    assert_clean_after_commit(&dir, &["rebase", "--continue"]);
}

#[test]
fn a_record_that_also_conflicts_in_its_body_is_refused_and_left_untouched() {
    let dir = fixture("body", Some("Body from x."), Some("Body from y."));
    assert!(!git(&dir, &["merge", "-q", "main"]));
    let before = std::fs::read_to_string(dir.join(REC)).unwrap();
    let (code, v) = apply(&dir);
    assert_eq!(
        (code, v["status"].as_str()),
        (Some(1), Some("partial-failure")),
        "{v}"
    );
    assert_eq!(v["conflicts"][0]["outcome"], "refused", "{v}");
    assert!(
        v["conflicts"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("Body from"),
        "{v}"
    );
    assert_eq!(std::fs::read_to_string(dir.join(REC)).unwrap(), before);
}

#[test]
fn a_record_whose_pinned_file_is_itself_unmerged_is_refused() {
    let dir = fixture("pinned", None, None);
    // Both branches also touch the same line of `src/a.rs`.
    ok(&dir, &["checkout", "-q", "main"]);
    std::fs::write(dir.join("src/a.rs"), "main\n").unwrap();
    commit_all(&dir, "main a");
    ok(&dir, &["checkout", "-q", "y"]);
    std::fs::write(dir.join("src/a.rs"), "y\n").unwrap();
    commit_all(&dir, "y a");
    assert!(!git(&dir, &["merge", "-q", "main"]));
    let before = std::fs::read_to_string(dir.join(REC)).unwrap();
    let (code, v) = apply(&dir);
    assert_eq!(code, Some(1), "{v}");
    let reason = v["conflicts"][0]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("itself unmerged"), "{v}");
    assert_eq!(std::fs::read_to_string(dir.join(REC)).unwrap(), before);
}

#[test]
fn a_tree_with_nothing_unmerged_says_so_distinctly() {
    let dir = fixture("none", None, None);
    let (code, v) = urzua(&dir, &["fix", "--conflicts"]);
    assert_eq!(
        (code, v["status"].as_str()),
        (Some(0), Some("nothing-conflicted")),
        "{v}"
    );
    assert_eq!(v["conflicts"].as_array().map(Vec::len), Some(0), "{v}");
}

fn assert_refused_untouched(dir: &Path, needle: &str) {
    let before = std::fs::read_to_string(dir.join(REC)).unwrap();
    let (code, v) = apply(dir);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(v["conflicts"][0]["outcome"], "refused", "{v}");
    let reason = v["conflicts"][0]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains(needle), "{needle}: {v}");
    assert!(reason.contains("fix --rebaseline --apply"), "{v}");
    assert_eq!(std::fs::read_to_string(dir.join(REC)).unwrap(), before);
}

#[test]
fn a_hand_edit_to_the_pinned_file_during_the_merge_is_not_pinned() {
    let dir = fixture("backdoor", None, None);
    assert!(!git(&dir, &["merge", "-q", "main"]));
    std::fs::write(dir.join("src/a.rs"), "fn backdoor() {}\n").unwrap();
    assert_refused_untouched(&dir, "working tree");
}

#[test]
fn a_staged_hand_edit_to_the_pinned_file_is_not_pinned() {
    let dir = fixture("staged-backdoor", None, None);
    assert!(!git(&dir, &["merge", "-q", "main"]));
    std::fs::write(dir.join("src/a.rs"), "fn backdoor() {}\n").unwrap();
    ok(&dir, &["add", "src/a.rs"]);
    assert_refused_untouched(&dir, "automatic merge");
}

#[test]
fn a_change_to_the_pinned_file_on_main_without_a_re_pin_is_not_laundered() {
    let dir = fixture("unpinned-main", None, None);
    ok(&dir, &["checkout", "-q", "main"]);
    let mut a = std::fs::read_to_string(dir.join("src/a.rs")).unwrap();
    a.push_str("fn unreviewed() {}\n");
    std::fs::write(dir.join("src/a.rs"), a).unwrap();
    commit_all(&dir, "main edits a.rs, no re-pin");
    ok(&dir, &["checkout", "-q", "y"]);
    assert!(!git(&dir, &["merge", "-q", "main"]));
    assert_refused_untouched(&dir, "their commit");
}

#[test]
fn a_second_run_after_a_resolve_reports_it_resolved_not_hand_resolved() {
    let dir = fixture("rerun", None, None);
    assert!(!git(&dir, &["merge", "-q", "main"]));
    assert_eq!(apply(&dir).0, Some(0));
    let (code, v) = urzua(&dir, &["fix", "--conflicts"]);
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["conflicts"][0]["outcome"], "resolved", "{v}");
    assert!(
        v["next_action"].as_str().unwrap().starts_with("git add"),
        "{v}"
    );
}

#[test]
fn check_base_lists_each_pin_recomputed_on_merge() {
    let dir = fixture("notice", None, None);
    assert!(!git(&dir, &["merge", "-q", "main"]));
    assert_eq!(apply(&dir).0, Some(0));
    ok(&dir, &["add", REC]);
    ok(&dir, &["commit", "-q", "--no-edit"]);
    let (_, v) = urzua(&dir, &["check", "--base", "main"]);
    let notice = v["notices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["subject"] == "pins-recomputed-on-merge")
        .unwrap_or_else(|| panic!("{v}"));
    let text = notice.to_string();
    assert!(text.contains(REC) && text.contains("src/a.rs"), "{text}");
}

const BUMP_ALL: &str = "schema_version: 2\nrules: {embodiment.consistency: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    revision_log: {marker: \"**Revision log**\", version_bump: {field: Version, classes: [substantive, structural]}}\n";
const BUMP_EVIDENCE: &str = "schema_version: 2\nrules: {embodiment.consistency: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    revision_log: {marker: \"**Revision log**\", classes: [substantive, structural, evidence], evidence_class: evidence, version_bump: {field: Version, classes: [substantive, structural]}}\n";

#[test]
fn without_an_evidence_class_a_resolve_bumps_the_version_check_base_requires() {
    let dir = fixture_with("bump", BUMP_ALL, ("2", "2", "structural"), (None, None));
    assert!(!git(&dir, &["merge", "-q", "main"]));
    let (code, v) = apply(&dir);
    assert_eq!(code, Some(0), "{v}");
    let text = std::fs::read_to_string(dir.join(REC)).unwrap();
    assert!(text.contains("Version: '3'"), "{text}");
    ok(&dir, &["add", REC]);
    ok(&dir, &["commit", "-q", "--no-edit"]);
    let (code, v) = urzua(&dir, &["check", "--base", "main"]);
    assert_eq!(code, Some(0), "{v}");
}

#[test]
fn with_an_evidence_class_a_resolve_writes_it_and_needs_no_bump() {
    let dir = fixture_with(
        "evidence",
        BUMP_EVIDENCE,
        ("1", "1", "evidence"),
        (None, None),
    );
    assert!(!git(&dir, &["merge", "-q", "main"]));
    let (code, v) = apply(&dir);
    assert_eq!(code, Some(0), "{v}");
    let text = std::fs::read_to_string(dir.join(REC)).unwrap();
    assert!(text.contains("Version: '1'"), "{text}");
    let merge_row = text
        .lines()
        .find(|l| l.contains("Pin recomputed on merge"))
        .unwrap();
    assert!(merge_row.ends_with("| **evidence** |"), "{merge_row}");
    ok(&dir, &["add", REC]);
    ok(&dir, &["commit", "-q", "--no-edit"]);
    let (code, v) = urzua(&dir, &["check", "--base", "main"]);
    assert_eq!(code, Some(0), "{v}");
}

#[test]
fn a_record_under_a_path_gate_is_refused_and_left_unresolved() {
    let config = format!(
        "{CONFIG}authority:\n  deciders:\n    leads: {{logins: [lead]}}\n  gates:\n    - {{paths: [{REC}], by: leads}}\n"
    );
    let dir = fixture_with("gated", &config, ("2", "3", "structural"), (None, None));
    assert!(!git(&dir, &["merge", "-q", "main"]), "the merge conflicts");
    let before = std::fs::read_to_string(dir.join(REC)).unwrap();
    let (_, v) = apply(&dir);
    assert_eq!(v["conflicts"][0]["outcome"], "refused", "{v}");
    let reason = v["conflicts"][0]["reason"].as_str().unwrap();
    assert!(
        reason.contains("path gate") && reason.contains("'leads' (lead)"),
        "{reason}"
    );
    assert_eq!(std::fs::read_to_string(dir.join(REC)).unwrap(), before);
}
