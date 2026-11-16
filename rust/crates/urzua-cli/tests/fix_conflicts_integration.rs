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
fn a_record_whose_pinned_file_is_still_unmerged_says_so_and_orders_the_work() {
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
    let c = &v["conflicts"][0];
    let reason = c["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("still unmerged (src/a.rs)"), "{v}");
    assert!(reason.contains("Resolve and stage that file first"), "{v}");
    assert!(!reason.contains("--rebaseline"), "{v}");
    assert_eq!(c["waits_on"], serde_json::json!(["src/a.rs"]), "{v}");
    assert_eq!(v["next_action"], "urzua fix --conflicts", "{v}");
    let why = v["next_action_reason"].as_str().unwrap();
    assert!(
        why.contains("src/a.rs") && why.contains("stage them with git add first"),
        "{v}"
    );
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

// Milestone records whose Done-means items carry pinned evidence clauses.

const MILE_CONFIG: &str = "schema_version: 2
rules: {embodiment.consistency: error, status.delivery-agreement: error, status.done-items-shape: error}

record_types:
  milestone:
    dir: \"docs/milestones\"
    required_fields: [Status]
    sections:
      from: headings
      required:
        - {name: Done means}
    statuses:
      values: [Planned, InProgress, Done]
      success: [Done]
      progress: [InProgress, Done]
      delivery:
        delivered_tiers: [Implemented, Verified]
        unevidenced_tiers: [\"Not started\"]
        not_done: [Planned, InProgress]
        set_status: Done
        items:
          section: \"Done means\"
          clause_separator: \" — \"
          evidence_marker: \"evidence:\"
          waiver_marker: \"waived:\"
          partial_status: \"InProgress\"
          resolving_tiers: [Specified, Implemented, Verified]
    revision_log: {marker: \"**Revision log**\", version_bump: {field: Version, classes: [substantive, structural]}}
";
const MREC: &str = "docs/milestones/MILE-1-a.md";
const B_RS: &str = "fn h() {\n    1;\n}\n\nfn spacer() {}\nfn spacer2() {}\nfn spacer3() {}\n\nfn k() {\n    1;\n}\n";

fn pin_of(dir: &Path, file: &str) -> String {
    sha256(&std::fs::read_to_string(dir.join(file)).unwrap())
}

/// A milestone pinning `src/a.rs` in `Realized-by` and `items` as its Done
/// means, each `{a}`/`{b}` replaced by that file's pin.
fn milestone(n: usize, dir: &Path, items: &str) -> String {
    let (a, b) = (pin_of(dir, "src/a.rs"), pin_of(dir, "src/b.rs"));
    let items = items.replace("{a}", &a).replace("{b}", &b);
    format!(
        "---\nStatus: Done\nVersion: '1'\nEmbodiment: Implemented\nRealized-by: 'code:src/a.rs@sha256:{a}, test:t.rs'\n---\n# MILE-{n}: T\n\n## Done means\n\n{items}\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
    )
}

const TWO_PINS: &str =
    "- Both files work. — evidence: code:src/a.rs@sha256:{a}, test:src/b.rs@sha256:{b}\n";
const TWO_ITEMS: &str = "- A works. — evidence: code:src/a.rs@sha256:{a}\n- B works. — evidence: code:src/b.rs@sha256:{b}\n";

/// `file` with its first (`which` 0) or last (`which` 1) `    1;` replaced.
fn edit(dir: &Path, file: &str, which: usize, to: &str) {
    let text = std::fs::read_to_string(dir.join(file)).unwrap();
    let at = if which == 0 {
        text.find("    ")
    } else {
        text.rfind("    ")
    }
    .unwrap();
    let end = at + text[at..].find(';').unwrap();
    std::fs::write(
        dir.join(file),
        format!("{}    {to}{}", &text[..at], &text[end..]),
    )
    .unwrap();
}

/// Every pin of `rec` re-pinned to its file as it is, `Version` set and one
/// row appended.
fn repin_all(dir: &Path, rec: &str, version: &str, row: &str) {
    let text = std::fs::read_to_string(dir.join(rec)).unwrap();
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some(at) = rest.find("@sha256:") {
        let start = rest[..at].rfind([':', ' ']).unwrap() + 1;
        let path = &rest[start..at];
        out.push_str(&rest[..at + 8]);
        out.push_str(&pin_of(dir, path.split('#').next().unwrap()));
        rest = &rest[at + 8 + 64..];
    }
    out.push_str(rest);
    let at = out.find("Version: '").unwrap() + 10;
    let end = at + out[at..].find('\'').unwrap();
    let out = format!("{}{version}{}{row}\n", &out[..at], &out[end..]);
    std::fs::write(dir.join(rec), out).unwrap();
}

/// main holds `records`; branch x runs `on_x`, branch y runs `on_y`; main
/// then fast-forwards to x and y is checked out.
fn mfixture(
    name: &str,
    records: &[(String, String)],
    on_x: &dyn Fn(&Path),
    on_y: &dyn Fn(&Path),
) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-conflicts-m-{name}"));
    for d in ["docs/milestones", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    ok(&dir, &["init", "-q", "-b", "main"]);
    ok(&dir, &["config", "user.email", "test@example.com"]);
    ok(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), MILE_CONFIG).unwrap();
    std::fs::write(dir.join("t.rs"), "t\n").unwrap();
    std::fs::write(dir.join("src/a.rs"), A_RS).unwrap();
    std::fs::write(dir.join("src/b.rs"), B_RS).unwrap();
    for (path, items) in records {
        let n: usize = path.split('-').nth(1).unwrap().parse().unwrap();
        let text = milestone(n, &dir, items);
        std::fs::write(dir.join(path), text).unwrap();
    }
    commit_all(&dir, "base");
    for (branch, on) in [("x", on_x), ("y", on_y)] {
        ok(&dir, &["checkout", "-q", "main"]);
        ok(&dir, &["checkout", "-q", "-b", branch]);
        on(&dir);
        commit_all(&dir, branch);
    }
    ok(&dir, &["checkout", "-q", "main"]);
    ok(&dir, &["merge", "-q", "--ff-only", "x"]);
    ok(&dir, &["checkout", "-q", "y"]);
    dir
}

/// One milestone with `items`; x edits the first function of both files, y
/// the last, each re-pinning with a row of its own.
fn one_milestone(name: &str, items: &str, x_row: &str, y_row: &str) -> common::TempDir {
    let (x_row, y_row) = (x_row.to_string(), y_row.to_string());
    mfixture(
        name,
        &[(MREC.to_string(), items.to_string())],
        &move |d| {
            edit(d, "src/a.rs", 0, "2;");
            edit(d, "src/b.rs", 0, "2;");
            repin_all(d, MREC, "2", &x_row);
        },
        &move |d| {
            edit(d, "src/a.rs", 1, "3;");
            edit(d, "src/b.rs", 1, "3;");
            repin_all(d, MREC, "2", &y_row);
        },
    )
}

const X_ROW: &str =
    "> | 2026-03-01 | Re-baselined by x: re-pinned both files. Bumped to 2. | **structural** |";
const Y_ROW: &str =
    "> | 2026-03-01 | Re-baselined by y: re-pinned both files. Bumped to 2. | **structural** |";

fn mapply(dir: &Path) -> serde_json::Value {
    let (code, v) = apply(dir);
    assert_eq!(
        (code, v["status"].as_str()),
        (Some(0), Some("resolved")),
        "{v}"
    );
    v
}

/// The milestone holds each file's merged pin at every site and nothing else
/// of the conflict.
fn assert_milestone_resolved(dir: &Path, rec: &str) -> String {
    let text = std::fs::read_to_string(dir.join(rec)).unwrap();
    assert!(
        !text.contains("<<<<<<<") && !text.contains("URZUA-MERGED"),
        "{text}"
    );
    let (a, b) = (pin_of(dir, "src/a.rs"), pin_of(dir, "src/b.rs"));
    for line in text.lines().filter(|l| l.contains("@sha256:")) {
        for (file, pin) in [("src/a.rs", &a), ("src/b.rs", &b)] {
            if line.contains(&format!("{file}@sha256:")) {
                assert!(line.contains(&format!("{file}@sha256:{pin}")), "{line}");
            }
        }
    }
    text
}

fn assert_mclean(dir: &Path, rec: &str, continue_args: &[&str]) {
    ok(dir, &["add", rec]);
    ok(dir, continue_args);
    let (code, v) = urzua(dir, &["check"]);
    assert_eq!(code, Some(0), "{v}");
    let (code, v) = urzua(dir, &["check", "--base", "main"]);
    assert_eq!(code, Some(0), "{v}");
    let (code, v) = urzua(dir, &["fix", "--rebaseline"]);
    assert_eq!((code, v["status"].as_str()), (Some(0), Some("ok")), "{v}");
}

#[test]
fn a_done_means_item_with_two_pins_and_the_same_pin_in_realized_by_resolves_on_merge() {
    let dir = one_milestone("item-merge", TWO_PINS, X_ROW, Y_ROW);
    assert!(!git(&dir, &["merge", "-q", "main"]), "the merge conflicts");
    let (code, v) = urzua(&dir, &["fix", "--conflicts"]);
    assert_eq!(
        (code, v["status"].as_str()),
        (Some(0), Some("resolvable")),
        "{v}"
    );
    let v = mapply(&dir);
    let pins = v["conflicts"][0]["pins"].as_array().unwrap();
    let sites: Vec<&str> = pins.iter().map(|p| p["site"].as_str().unwrap()).collect();
    assert_eq!(
        sites,
        ["Realized-by", "Done means item 1", "Done means item 1"],
        "{v}"
    );
    assert!(pins.iter().all(|p| p["recomputed"] == true), "{v}");
    let text = assert_milestone_resolved(&dir, MREC);
    assert!(text.contains("(Done means item 1)"), "{text}");
    assert_mclean(&dir, MREC, &["commit", "-q", "--no-edit"]);
}

#[test]
fn a_done_means_item_with_two_pins_resolves_on_rebase() {
    let dir = one_milestone("item-rebase", TWO_PINS, X_ROW, Y_ROW);
    assert!(
        !git(&dir, &["rebase", "-q", "main"]),
        "the rebase conflicts"
    );
    mapply(&dir);
    assert_milestone_resolved(&dir, MREC);
    assert_mclean(&dir, MREC, &["rebase", "--continue"]);
}

#[test]
fn two_done_means_items_each_pinning_a_file_resolve_on_merge_and_rebase() {
    for (name, op, cont) in [
        ("items-merge", "merge", &["commit", "-q", "--no-edit"][..]),
        ("items-rebase", "rebase", &["rebase", "--continue"][..]),
    ] {
        let dir = one_milestone(name, TWO_ITEMS, X_ROW, Y_ROW);
        assert!(!git(&dir, &[op, "-q", "main"]), "{op} conflicts");
        let v = mapply(&dir);
        let sites: Vec<&str> = v["conflicts"][0]["pins"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["site"].as_str().unwrap())
            .collect();
        assert_eq!(
            sites,
            ["Realized-by", "Done means item 1", "Done means item 2"],
            "{v}"
        );
        assert_milestone_resolved(&dir, MREC);
        assert_mclean(&dir, MREC, cont);
    }
}

#[test]
fn a_conflict_in_a_done_means_items_text_is_refused_naming_the_item() {
    let dir = mfixture(
        "item-text",
        &[(MREC.to_string(), TWO_PINS.to_string())],
        &|d| {
            edit(d, "src/a.rs", 0, "2;");
            repin_all(d, MREC, "2", X_ROW);
            let t = std::fs::read_to_string(d.join(MREC)).unwrap();
            std::fs::write(
                d.join(MREC),
                t.replace("Both files work.", "Both files work for x."),
            )
            .unwrap();
        },
        &|d| {
            edit(d, "src/a.rs", 1, "3;");
            repin_all(d, MREC, "2", Y_ROW);
            let t = std::fs::read_to_string(d.join(MREC)).unwrap();
            std::fs::write(
                d.join(MREC),
                t.replace("Both files work.", "Both files work for y."),
            )
            .unwrap();
        },
    );
    assert!(!git(&dir, &["merge", "-q", "main"]));
    let before = std::fs::read_to_string(dir.join(MREC)).unwrap();
    let (code, v) = apply(&dir);
    assert_eq!(code, Some(1), "{v}");
    let reason = v["conflicts"][0]["reason"].as_str().unwrap();
    assert!(reason.contains("Done means item 1"), "{v}");
    assert_eq!(std::fs::read_to_string(dir.join(MREC)).unwrap(), before);
}

#[test]
fn a_one_sided_change_to_an_items_text_while_its_pins_moved_is_refused() {
    let dir = mfixture(
        "item-one-sided",
        &[(MREC.to_string(), TWO_PINS.to_string())],
        &|d| {
            edit(d, "src/a.rs", 0, "2;");
            repin_all(d, MREC, "2", X_ROW);
            let t = std::fs::read_to_string(d.join(MREC)).unwrap();
            std::fs::write(
                d.join(MREC),
                t.replace("Both files work.", "Both files work well."),
            )
            .unwrap();
        },
        &|d| {
            edit(d, "src/a.rs", 1, "3;");
            repin_all(d, MREC, "2", Y_ROW);
        },
    );
    assert!(!git(&dir, &["merge", "-q", "main"]));
    let (code, v) = apply(&dir);
    assert_eq!(code, Some(1), "{v}");
    let reason = v["conflicts"][0]["reason"].as_str().unwrap();
    assert!(reason.contains("Done means item 1 differs"), "{v}");
}

#[test]
fn rows_of_one_date_put_the_merged_in_side_first_and_the_version_row_is_not_left_false() {
    for (name, op, cont) in [
        ("order-merge", "merge", &["commit", "-q", "--no-edit"][..]),
        ("order-rebase", "rebase", &["rebase", "--continue"][..]),
    ] {
        let dir = one_milestone(name, TWO_PINS, X_ROW, Y_ROW);
        assert!(!git(&dir, &[op, "-q", "main"]), "{op} conflicts");
        let v = mapply(&dir);
        assert_eq!(v["conflicts"][0]["version"], "'3'", "{v}");
        let text = std::fs::read_to_string(dir.join(MREC)).unwrap();
        let x = text.find("Re-baselined by x").unwrap();
        let y = text.find("Re-baselined by y").unwrap();
        assert!(x < y, "{op}: main's row before this branch's: {text}");
        assert!(
            text.contains("Bumped to 2."),
            "a person's row is not rewritten: {text}"
        );
        let merge_row = text
            .lines()
            .find(|l| l.contains("Pin recomputed on merge"))
            .unwrap();
        assert!(
            merge_row.contains("`Version` is now 3; an earlier row's version 2"),
            "{merge_row}"
        );
        assert_mclean(&dir, MREC, cont);
    }
}

#[test]
fn hand_written_index_stages_are_refused() {
    let dir = one_milestone("forged", TWO_PINS, X_ROW, Y_ROW);
    assert!(!git(&dir, &["merge", "-q", "main"]));
    let theirs = Command::new("git")
        .args(["show", &format!(":3:{MREC}")])
        .current_dir(&*dir)
        .output()
        .unwrap()
        .stdout;
    let forged = String::from_utf8(theirs)
        .unwrap()
        .replace("Re-baselined by x", "Re-baselined by z");
    std::fs::write(dir.join("forged.md"), forged).unwrap();
    let blob = Command::new("git")
        .args(["hash-object", "-w", "forged.md"])
        .current_dir(&*dir)
        .output()
        .unwrap()
        .stdout;
    let blob = String::from_utf8(blob).unwrap().trim().to_string();
    std::fs::remove_file(dir.join("forged.md")).unwrap();
    let mut child = Command::new("git")
        .args(["update-index", "--index-info"])
        .current_dir(&*dir)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("100644 {blob} 3\t{MREC}\n").as_bytes())
        .unwrap();
    assert!(child.wait().unwrap().success());
    let (code, v) = apply(&dir);
    assert_eq!(code, Some(1), "{v}");
    assert!(
        v["conflicts"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("index stage 3"),
        "{v}"
    );
}

fn refused_with(dir: &Path, needle: &str) {
    let (code, v) = apply(dir);
    assert_eq!(code, Some(1), "{v}");
    let reason = v["conflicts"][0]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains(needle), "{needle}: {v}");
}

#[test]
fn a_pinned_file_that_is_a_symlink_or_changed_mode_is_refused() {
    let dir = one_milestone("symlink", TWO_ITEMS, X_ROW, Y_ROW);
    assert!(!git(&dir, &["merge", "-q", "main"]));
    let a = dir.join("src/a.rs");
    std::fs::copy(&a, dir.join("a.copy")).unwrap();
    std::fs::remove_file(&a).unwrap();
    std::os::unix::fs::symlink("../a.copy", &a).unwrap();
    refused_with(&dir, "is a symlink in the working tree");

    let dir = mfixture(
        "mode",
        &[(MREC.to_string(), TWO_ITEMS.to_string())],
        &|d| {
            edit(d, "src/a.rs", 0, "2;");
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(d.join("src/a.rs"), std::fs::Permissions::from_mode(0o755))
                .unwrap();
            repin_all(d, MREC, "2", X_ROW);
        },
        &|d| {
            edit(d, "src/a.rs", 1, "3;");
            repin_all(d, MREC, "2", Y_ROW);
        },
    );
    assert!(!git(&dir, &["merge", "-q", "main"]));
    refused_with(&dir, "changed its file mode");
}

#[test]
fn a_line_ending_conversion_is_named_not_blamed_on_a_hand_edit() {
    let dir = one_milestone("crlf", TWO_ITEMS, X_ROW, Y_ROW);
    ok(&dir, &["config", "core.autocrlf", "true"]);
    assert!(!git(&dir, &["merge", "-q", "main"]));
    let (code, v) = apply(&dir);
    assert_eq!(code, Some(1), "{v}");
    let reason = v["conflicts"][0]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("only in line endings"), "{v}");
    assert!(!reason.contains("edited by hand"), "{v}");
}

#[test]
fn a_later_stop_of_a_multi_commit_rebase_where_both_sides_re_pinned_resolves() {
    let dir = mfixture(
        "multi",
        &[(MREC.to_string(), TWO_ITEMS.to_string())],
        &|d| {
            edit(d, "src/a.rs", 0, "2;");
            repin_all(
                d,
                MREC,
                "2",
                "> | 2026-03-01 | Re-baselined by x. | **structural** |",
            );
        },
        &|d| {
            edit(d, "src/a.rs", 1, "3;");
            repin_all(
                d,
                MREC,
                "2",
                "> | 2026-03-02 | Re-baselined by y1. | **structural** |",
            );
            commit_all(d, "y1");
            edit(d, "src/a.rs", 1, "4;");
            repin_all(
                d,
                MREC,
                "3",
                "> | 2026-03-03 | Re-baselined by y2. | **structural** |",
            );
        },
    );
    assert!(
        !git(&dir, &["rebase", "-q", "main"]),
        "the first stop conflicts"
    );
    mapply(&dir);
    ok(&dir, &["add", MREC]);
    assert!(
        !git(&dir, &["rebase", "--continue"]),
        "the second stop conflicts"
    );
    let v = mapply(&dir);
    assert_eq!(v["conflicts"][0]["pins"][1]["recomputed"], true, "{v}");
    let text = assert_milestone_resolved(&dir, MREC);
    let at = |s: &str| text.find(s).unwrap_or_else(|| panic!("{s}: {text}"));
    assert!(
        at("by x.") < at("by y1.") && at("by y1.") < at("by y2."),
        "{text}"
    );
    assert_mclean(&dir, MREC, &["rebase", "--continue"]);
}

#[test]
fn thirty_milestones_both_branches_re_pinned_all_resolve_in_one_run() {
    let recs: Vec<(String, String)> = (1..=30)
        .map(|n| {
            (
                format!("docs/milestones/MILE-{n}-m.md"),
                TWO_PINS.to_string(),
            )
        })
        .collect();
    let paths: Vec<String> = recs.iter().map(|(p, _)| p.clone()).collect();
    let (px, py) = (paths.clone(), paths.clone());
    let dir = mfixture(
        "thirty",
        &recs,
        &move |d| {
            edit(d, "src/a.rs", 0, "2;");
            edit(d, "src/b.rs", 0, "2;");
            for p in &px {
                repin_all(d, p, "2", X_ROW);
            }
        },
        &move |d| {
            edit(d, "src/a.rs", 1, "3;");
            edit(d, "src/b.rs", 1, "3;");
            for p in &py {
                repin_all(d, p, "2", Y_ROW);
            }
        },
    );
    assert!(!git(&dir, &["merge", "-q", "main"]));
    let v = mapply(&dir);
    let resolved = v["conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["outcome"] == "resolved")
        .count();
    assert_eq!(resolved, 30, "{v}");
    for p in &paths {
        assert_milestone_resolved(&dir, p);
        ok(&dir, &["add", p]);
    }
    ok(&dir, &["commit", "-q", "--no-edit"]);
    let (code, v) = urzua(&dir, &["check"]);
    assert_eq!(code, Some(0), "{v}");
    let (code, v) = urzua(&dir, &["fix", "--rebaseline"]);
    assert_eq!((code, v["status"].as_str()), (Some(0), Some("ok")), "{v}");
}
