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
    assert_eq!(v["next_action"], "git diff -- :/src/a.rs", "{v}");
    assert_eq!(v["next_action_inputs"], serde_json::json!([]), "{v}");
    let why = v["next_action_reason"].as_str().unwrap();
    assert!(
        why.contains("`git add -- :/src/a.rs`, then `urzua fix --conflicts`"),
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

#[test]
fn an_ids_entry_matching_nothing_in_a_tree_with_nothing_unmerged_is_a_failure() {
    let dir = fixture("none-ids", None, None);
    let (code, v) = urzua(
        &dir,
        &["fix", "--conflicts", "--ids", "docs/adr/ADR-1-x.md"],
    );
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(v["status"], "partial-failure", "{v}");
    let failed = v["failed"].as_array().unwrap_or_else(|| panic!("{v}"));
    assert_eq!(failed.len(), 1, "{v}");
    assert_eq!(failed[0]["record"], "docs/adr/ADR-1-x.md", "{v}");
    assert!(
        failed[0]["error"]
            .as_str()
            .unwrap()
            .contains("--ids entry 'docs/adr/ADR-1-x.md' matched no unmerged record"),
        "{v}"
    );
    assert_eq!(v["next_action"], "urzua fix --conflicts", "{v}");
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
/// A line `RB: <value>` in `items` replaces the `Realized-by` value and a
/// line `ROW: <row>` adds a base row after the first.
fn milestone(n: usize, dir: &Path, items: &str) -> String {
    let (a, b) = (pin_of(dir, "src/a.rs"), pin_of(dir, "src/b.rs"));
    let l13 = sha256(
        &std::fs::read_to_string(dir.join("src/a.rs"))
            .unwrap()
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let l110 = sha256(
        &std::fs::read_to_string(dir.join("src/a.rs"))
            .unwrap()
            .lines()
            .take(10)
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let l57 = sha256(
        &std::fs::read_to_string(dir.join("src/a.rs"))
            .unwrap()
            .lines()
            .skip(4)
            .take(3)
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let items = items
        .replace("{l57}", &l57)
        .replace("{l110}", &l110)
        .replace("{a}", &a)
        .replace("{b}", &b)
        .replace("{l13}", &l13);
    let mut rb = format!("code:src/a.rs@sha256:{a}, test:t.rs");
    let mut rows = String::new();
    let mut body = String::new();
    for line in items.lines() {
        if let Some(v) = line.strip_prefix("RB: ") {
            rb = v.to_string();
        } else if let Some(r) = line.strip_prefix("ROW: ") {
            rows.push_str(r);
            rows.push('\n');
        } else {
            body.push_str(line);
            body.push('\n');
        }
    }
    format!(
        "---\nStatus: Done\nVersion: '1'\nEmbodiment: Implemented\nRealized-by: '{rb}'\n---\n# MILE-{n}: T\n\n## Done means\n\n{body}\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n{rows}"
    )
}

/// The Done-means item lines of `text` with every pin hash blanked.
fn items_without_hashes(text: &str) -> Vec<String> {
    let section = text
        .split("## Done means")
        .nth(1)
        .unwrap()
        .split("> **Revision log**")
        .next()
        .unwrap();
    section
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let mut out = String::new();
            let mut rest = l;
            while let Some(at) = rest.find("@sha256:") {
                out.push_str(&rest[..at + 8]);
                rest = &rest[at + 8 + 64..];
            }
            out.push_str(rest);
            out
        })
        .collect()
}

fn show(dir: &Path, spec: &str) -> String {
    let out = Command::new("git")
        .args(["show", spec])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap()
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
    mfixture_with(name, MILE_CONFIG, records, on_x, on_y)
}

fn mfixture_with(
    name: &str,
    config: &str,
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
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
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
            merge_row.contains(
                "`Version` is set to 3 by this merge, the next number above the higher side's 2"
            ) && merge_row.contains("states version 2, which was true there"),
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

/// x re-pins once, y twice in two commits.
fn multi_fixture(name: &str) -> common::TempDir {
    mfixture(
        name,
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
    )
}

#[test]
fn a_later_stop_of_a_multi_commit_rebase_where_both_sides_re_pinned_resolves() {
    let dir = multi_fixture("multi");
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
    let merged = multi_fixture("multi-as-merge");
    assert!(!git(&merged, &["merge", "-q", "main"]));
    mapply(&merged);
    let person_only = |t: String| -> String {
        t.lines()
            .filter(|l| !l.contains("on merge (attested by"))
            .map(|l| format!("{l}\n"))
            .collect()
    };
    assert_eq!(
        person_only(std::fs::read_to_string(dir.join(MREC)).unwrap()),
        person_only(std::fs::read_to_string(merged.join(MREC)).unwrap()),
        "the rebased record equals the merged one, machine rows aside"
    );
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
    for p in &paths {
        let result = items_without_hashes(&show(&dir, &format!("HEAD:{p}")));
        for side in ["HEAD^1", "HEAD^2"] {
            assert_eq!(
                result,
                items_without_hashes(&show(&dir, &format!("{side}:{p}"))),
                "{p} {side}"
            );
        }
    }
    let (code, v) = urzua(&dir, &["check"]);
    assert_eq!(code, Some(0), "{v}");
    let (code, v) = urzua(&dir, &["fix", "--rebaseline"]);
    assert_eq!((code, v["status"].as_str()), (Some(0), Some("ok")), "{v}");
}

/// Re-pins the `nth` occurrence (from 0) of `file`'s pin in `rec`.
fn repin_nth(dir: &Path, rec: &str, file: &str, nth: usize) {
    let text = std::fs::read_to_string(dir.join(rec)).unwrap();
    let needle = format!("{file}@sha256:");
    let at = text.match_indices(&needle).nth(nth).unwrap().0 + needle.len();
    let out = format!("{}{}{}", &text[..at], pin_of(dir, file), &text[at + 64..]);
    std::fs::write(dir.join(rec), out).unwrap();
}

fn add_row(dir: &Path, rec: &str, version: &str, row: &str) {
    let text = std::fs::read_to_string(dir.join(rec)).unwrap();
    let text = text.replace("Version: '1'", &format!("Version: '{version}'"));
    std::fs::write(dir.join(rec), format!("{text}{row}\n")).unwrap();
}

/// Both branches edit `src/a.rs` and re-pin occurrences `repinned` of its pin.
fn repeated(name: &str, items: &str, repinned: &'static [usize]) -> common::TempDir {
    mfixture(
        name,
        &[(MREC.to_string(), items.to_string())],
        &move |d| {
            edit(d, "src/a.rs", 0, "2;");
            for n in repinned {
                repin_nth(d, MREC, "src/a.rs", *n);
            }
            add_row(d, MREC, "2", X_ROW);
        },
        &move |d| {
            edit(d, "src/a.rs", 1, "3;");
            for n in repinned {
                repin_nth(d, MREC, "src/a.rs", *n);
            }
            add_row(d, MREC, "2", Y_ROW);
        },
    )
}

#[test]
fn a_locator_repeated_in_one_item_clause_keeps_each_occurrences_own_pin() {
    let dir = repeated(
        "repeat-item",
        "- Twice. — evidence: code:src/a.rs@sha256:{a}, code:src/a.rs@sha256:{a}\n",
        &[0, 1],
    );
    let old = sha256(A_RS);
    assert!(!git(&dir, &["merge", "-q", "main"]));
    let v = mapply(&dir);
    let merged = pin_of(&dir, "src/a.rs");
    let text = std::fs::read_to_string(dir.join(MREC)).unwrap();
    let item = text.lines().find(|l| l.starts_with("- Twice.")).unwrap();
    assert_eq!(
        item,
        format!("- Twice. — evidence: code:src/a.rs@sha256:{merged}, code:src/a.rs@sha256:{old}"),
        "{v}"
    );
    let row = text
        .lines()
        .find(|l| l.contains("Pin recomputed on merge"))
        .unwrap();
    assert!(row.contains("(Done means item 1)"), "{row}");
}

#[test]
fn a_locator_repeated_in_realized_by_keeps_each_occurrences_own_pin() {
    let dir = repeated(
        "repeat-rb",
        "RB: code:src/a.rs@sha256:{a}, code:src/a.rs@sha256:{a}, test:t.rs\n- B works. — evidence: code:src/b.rs@sha256:{b}\n",
        &[0],
    );
    let old = sha256(A_RS);
    assert!(!git(&dir, &["merge", "-q", "main"]));
    mapply(&dir);
    let merged = pin_of(&dir, "src/a.rs");
    let text = std::fs::read_to_string(dir.join(MREC)).unwrap();
    assert!(
        text.contains(&format!(
            "Realized-by: 'code:src/a.rs@sha256:{merged}, code:src/a.rs@sha256:{old}, test:t.rs'"
        )),
        "{text}"
    );
}

#[test]
fn a_version_one_side_set_is_kept_and_no_false_version_row_is_written() {
    let dir = mfixture(
        "version-kept",
        &[(
            MREC.to_string(),
            format!("{TWO_ITEMS}ROW: > | 2026-02-01 | Reviewed. Bumped to 1 after review. | **structural** |\n"),
        )],
        &|d| {
            edit(d, "src/b.rs", 0, "2;");
            repin_all(d, MREC, "2", X_ROW);
        },
        &|d| {
            add_row(d, MREC, "1", "> | 2026-03-01 | Wording. | **editorial** |");
        },
    );
    assert!(!git(&dir, &["merge", "-q", "main"]), "the rows conflict");
    let v = mapply(&dir);
    assert_eq!(v["conflicts"][0]["version"], "'2'", "{v}");
    let text = std::fs::read_to_string(dir.join(MREC)).unwrap();
    assert!(!text.contains("on merge (attested by"), "{text}");
    assert!(text.contains("Version: '2'"), "{text}");
}

#[test]
fn a_hand_written_merge_base_stage_is_refused() {
    let dir = one_milestone("forged-base", TWO_PINS, X_ROW, Y_ROW);
    assert!(!git(&dir, &["merge", "-q", "main"]));
    let ours = Command::new("git")
        .args(["rev-parse", &format!(":2:{MREC}")])
        .current_dir(&*dir)
        .output()
        .unwrap()
        .stdout;
    let ours = String::from_utf8(ours).unwrap().trim().to_string();
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
        .write_all(format!("100644 {ours} 1\t{MREC}\n").as_bytes())
        .unwrap();
    assert!(child.wait().unwrap().success());
    let (code, v) = apply(&dir);
    assert_eq!(code, Some(1), "{v}");
    assert!(
        v["conflicts"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("index stage 1"),
        "{v}"
    );
}

#[test]
fn check_base_is_clean_after_a_resolve_when_version_bump_leaves_out_the_merge_rows_class() {
    let config = MILE_CONFIG.replace(
        "classes: [substantive, structural]",
        "classes: [substantive]",
    );
    let dir = one_milestone("no-bump-class", TWO_PINS, X_ROW, Y_ROW);
    for branch in ["main", "y"] {
        ok(&dir, &["checkout", "-q", branch]);
        std::fs::write(dir.join(".urzua/config.yaml"), &config).unwrap();
        commit_all(&dir, "config");
    }
    assert!(!git(&dir, &["merge", "-q", "main"]));
    let v = mapply(&dir);
    assert_eq!(v["conflicts"][0]["version"], "'2'", "{v}");
    assert_mclean(&dir, MREC, &["commit", "-q", "--no-edit"]);
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

#[test]
fn a_hand_merged_pinned_file_leads_to_a_sequence_that_passes_check_base() {
    let dir = fixture("hand-merged", None, None);
    assert!(!git(&dir, &["merge", "-q", "main"]));
    std::fs::write(dir.join("src/a.rs"), "fn merged_by_hand() {}\n").unwrap();
    ok(&dir, &["add", "src/a.rs"]);
    let (_, v) = urzua(&dir, &["fix", "--conflicts"]);
    let next = v["next_action"].as_str().unwrap_or_else(|| panic!("{v}"));
    assert_eq!(next, format!("git checkout --theirs -- :/{REC}"), "{v}");
    assert!(
        v["next_action_reason"]
            .as_str()
            .unwrap()
            .contains("urzua fix --rebaseline --apply --ids"),
        "{v}"
    );
    let out = Command::new("sh")
        .arg("-c")
        .arg(next)
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    ok(&dir, &["add", REC]);
    let (_, v) = urzua(
        &dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            REC,
            "--by",
            "tester",
            "--actor",
            "agent",
            "--reason",
            "a.rs was merged by hand; read the merged code and the claim holds.",
        ],
    );
    assert_eq!(v["status"], "applied", "{v}");
    ok(&dir, &["add", "-A"]);
    ok(&dir, &["commit", "-q", "--no-edit"]);
    let (_, v) = urzua(&dir, &["check", "--base", "main"]);
    let pins: Vec<_> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "diff.pin-changed-unattested")
        .collect();
    assert!(pins.is_empty(), "{v}");
}

#[test]
fn a_dot_slash_ids_entry_resolves_the_conflicted_record() {
    let dir = fixture("ids-dot-slash", None, None);
    assert!(!git(&dir, &["merge", "-q", "main"]), "the merge conflicts");
    let (code, v) = urzua(
        &dir,
        &[
            "fix",
            "--conflicts",
            "--apply",
            "--ids",
            &format!("./{REC}"),
            "--by",
            "tester",
            "--actor",
            "agent",
        ],
    );
    assert_eq!(
        (code, v["status"].as_str()),
        (Some(0), Some("resolved")),
        "{v}"
    );
    assert_resolved(&dir);
}

#[test]
fn an_unmatched_conflicts_entry_names_the_dry_run_that_lists_the_conflicts() {
    let dir = fixture("ids-unmatched", None, None);
    assert!(!git(&dir, &["merge", "-q", "main"]), "the merge conflicts");
    let (code, v) = urzua(
        &dir,
        &["fix", "--conflicts", "--ids", "docs/adr/ADR-9-x.md"],
    );
    assert_eq!(
        (code, v["status"].as_str()),
        (Some(1), Some("partial-failure")),
        "{v}"
    );
    assert_eq!(v["next_action"], "urzua fix --conflicts", "{v}");
}

/// main pins the whole of `src/a.rs`. Branch x narrows that locator to its
/// first three lines and pins them; branch y edits `f`, inside those lines, and
/// re-pins the whole file's old locator. y is checked out.
fn narrowed_locator_fixture(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-conflicts-{name}"));
    for d in ["docs/adr", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    ok(&dir, &["init", "-q", "-b", "main"]);
    ok(&dir, &["config", "user.email", "test@example.com"]);
    ok(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    std::fs::write(dir.join("docs/s.md"), "s\n").unwrap();
    std::fs::write(dir.join("t.rs"), "t\n").unwrap();
    std::fs::write(dir.join("src/a.rs"), A_RS).unwrap();
    std::fs::write(dir.join(REC), record(&sha256(A_RS), "1", "Body.")).unwrap();
    commit_all(&dir, "base");
    ok(&dir, &["checkout", "-q", "-b", "x"]);
    let (code, v) = urzua(
        &dir,
        &[
            "fix",
            "--pin",
            "--apply",
            "--ids",
            REC,
            "--replace",
            "src/a.rs=src/a.rs#L1-L3",
            "--by",
            "tester",
            "--actor",
            "agent",
            "--reason",
            "the claim rests on the first function only",
        ],
    );
    assert_eq!(code, Some(0), "{v}");
    commit_all(&dir, "x");
    ok(&dir, &["checkout", "-q", "main"]);
    ok(&dir, &["checkout", "-q", "-b", "y"]);
    std::fs::write(dir.join("src/a.rs"), A_RS.replacen("    1;", "    9;", 1)).unwrap();
    repin_as(&dir, "2026-03-01", "3", "y", None, "structural");
    commit_all(&dir, "y");
    dir
}

#[test]
fn a_locator_one_side_narrowed_and_the_other_re_pinned_resolves_to_the_narrowed_locator_observed_failing(
) {
    let dir = narrowed_locator_fixture("narrowed");
    assert!(!git(&dir, &["merge", "-q", "x"]), "the merge conflicts");
    let (code, v) = apply(&dir);
    assert_eq!(
        (code, v["status"].as_str()),
        (Some(0), Some("resolved")),
        "{v}"
    );
    let text = std::fs::read_to_string(dir.join(REC)).unwrap();
    let want = format!(
        "code:src/a.rs#L1-L3@sha256:{}",
        sha256("fn f() {\n    9;\n}")
    );
    assert!(text.contains(&want), "{text}");
    assert!(!text.contains("<<<<<<<"), "{text}");
    assert!(!text.contains("code:src/a.rs@sha256:"), "{text}");
    assert_clean_after_commit(&dir, &["commit", "-q", "--no-edit"]);
}

const NARROW_FIRST: &[&str] = &[
    "fix",
    "--pin",
    "--apply",
    "--ids",
    MREC,
    "--replace",
    "src/a.rs=src/a.rs#L1-L3",
    "--by",
    "tester",
    "--actor",
    "agent",
    "--reason",
    "first function only",
];

#[test]
fn a_done_means_item_locator_one_side_narrowed_and_the_other_re_pinned_resolves_observed_failing() {
    let dir = mfixture(
        "item-narrowed",
        &[(MREC.to_string(), TWO_PINS.to_string())],
        &|d| {
            let (code, v) = urzua(d, NARROW_FIRST);
            assert_eq!(code, Some(0), "{v}");
        },
        &|d| {
            edit(d, "src/a.rs", 0, "9;");
            edit(d, "src/b.rs", 1, "3;");
            let (code, v) = urzua(
                d,
                &[
                    "fix",
                    "--rebaseline",
                    "--apply",
                    "--ids",
                    MREC,
                    "--by",
                    "tester",
                    "--actor",
                    "agent",
                    "--reason",
                    "both functions changed",
                ],
            );
            assert_eq!(code, Some(0), "{v}");
        },
    );
    assert!(!git(&dir, &["merge", "-q", "main"]), "the merge conflicts");
    let v = mapply(&dir);
    let moved = v["conflicts"][0]["pins"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["moved_from"] == "src/a.rs" && p["recomputed"] == true)
        .count();
    assert_eq!(moved, 2, "{v}");
    let text = std::fs::read_to_string(dir.join(MREC)).unwrap();
    let first_three: Vec<String> = std::fs::read_to_string(dir.join("src/a.rs"))
        .unwrap()
        .lines()
        .take(3)
        .map(str::to_string)
        .collect();
    assert!(first_three[1].contains('9'), "{first_three:?}");
    let lines_1_3 = sha256(&first_three.join("\n"));
    let narrowed = format!("code:src/a.rs#L1-L3@sha256:{lines_1_3}");
    assert_eq!(text.matches(&narrowed).count(), 2, "{text}");
    assert!(!text.contains("code:src/a.rs@sha256:"), "{text}");
    assert!(text.contains("replaced `src/a.rs`"), "{text}");
    let b = pin_of(&dir, "src/b.rs");
    assert!(
        text.contains(&format!("test:src/b.rs@sha256:{b}")),
        "{text}"
    );
    assert_mclean(&dir, MREC, &["commit", "-q", "--no-edit"]);
}

#[test]
fn a_locator_both_sides_replaced_differently_is_refused_naming_both_observed_failing() {
    let dir = mfixture(
        "item-both",
        &[(MREC.to_string(), TWO_PINS.to_string())],
        &|d| {
            let (code, v) = urzua(d, NARROW_FIRST);
            assert_eq!(code, Some(0), "{v}");
        },
        &|d| {
            let mut args = NARROW_FIRST.to_vec();
            args[6] = "src/a.rs=src/a.rs#L9-L11";
            let (code, v) = urzua(d, &args);
            assert_eq!(code, Some(0), "{v}");
        },
    );
    assert!(!git(&dir, &["merge", "-q", "main"]), "the merge conflicts");
    let before = std::fs::read_to_string(dir.join(MREC)).unwrap();
    let (code, v) = apply(&dir);
    assert_eq!(code, Some(1), "{v}");
    let reason = v["conflicts"][0]["reason"].as_str().unwrap();
    assert!(
        reason.contains("src/a.rs#L1-L3") && reason.contains("src/a.rs#L9-L11"),
        "{v}"
    );
    assert_eq!(std::fs::read_to_string(dir.join(MREC)).unwrap(), before);
}

const ONE_FILE_ITEM: &str =
    "- A works. — evidence: code:src/a.rs@sha256:{a}\nRB: code:src/a.rs@sha256:{a}, test:t.rs\n";
const TWO_FILE_RB: &str = "- A works. — evidence: code:src/a.rs@sha256:{a}\nRB: code:src/b.rs@sha256:{b}, code:src/a.rs@sha256:{a}, test:t.rs\n";
const RANGE_ITEM: &str = "- A works. — evidence: code:src/a.rs#L1-L3@sha256:{l13}\nRB: code:src/a.rs#L1-L3@sha256:{l13}, test:t.rs\n";

fn pin_args<'a>(rec: &'a str, flag: &'a str, value: &'a str) -> Vec<&'a str> {
    vec![
        "fix",
        "--pin",
        "--apply",
        "--ids",
        rec,
        flag,
        value,
        "--by",
        "tester",
        "--actor",
        "agent",
        "--reason",
        "the claim is narrower",
    ]
}

fn run_ok(d: &Path, args: &[&str]) {
    let (code, v) = urzua(d, args);
    assert_eq!(code, Some(0), "{v}");
}

fn rebaseline(d: &Path) {
    run_ok(
        d,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            MREC,
            "--by",
            "tester",
            "--actor",
            "agent",
            "--reason",
            "the code changed",
        ],
    );
}

/// Replaces the pin that follows `needle` in `rec` with zeros, everywhere it
/// is written: a pin no file hashes to.
fn bogus_pin(d: &Path, needle: &str) {
    let text = std::fs::read_to_string(d.join(MREC)).unwrap();
    let at = text.find(needle).unwrap() + needle.len();
    let hash = text[at..at + 64].to_string();
    std::fs::write(d.join(MREC), text.replace(&hash, &"0".repeat(64))).unwrap();
}

fn moved(name: &str, items: &str, on_x: &dyn Fn(&Path), on_y: &dyn Fn(&Path)) -> common::TempDir {
    let dir = mfixture(name, &[(MREC.to_string(), items.to_string())], on_x, on_y);
    assert!(!git(&dir, &["merge", "-q", "main"]), "the merge conflicts");
    dir
}

/// `urzua fix --conflicts --apply` refuses the record, naming `needle`, and
/// leaves the file as the merge wrote it.
fn refused_untouched(dir: &Path, needle: &str) {
    let before = std::fs::read_to_string(dir.join(MREC)).unwrap();
    let (code, v) = apply(dir);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(v["conflicts"][0]["outcome"], "refused", "{v}");
    let reason = v["conflicts"][0]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains(needle), "{needle}: {v}");
    assert_eq!(std::fs::read_to_string(dir.join(MREC)).unwrap(), before);
}

fn y_edits_the_first_function_and_re_pins(d: &Path) {
    edit(d, "src/a.rs", 0, "9;");
    rebaseline(d);
}

#[test]
fn a_narrowing_whose_own_pin_matches_no_file_is_refused_observed_failing() {
    let dir = moved(
        "moved-own-pin",
        TWO_PINS,
        &|d| {
            run_ok(d, &pin_args(MREC, "--replace", "src/a.rs=src/a.rs#L1-L3"));
            bogus_pin(d, "src/a.rs#L1-L3@sha256:");
        },
        &y_edits_the_first_function_and_re_pins,
    );
    refused_untouched(&dir, "does not hash to that side's pin");
}

#[test]
fn a_re_pin_of_the_old_locator_that_matches_no_file_is_refused_observed_failing() {
    let dir = moved(
        "moved-other-pin",
        TWO_PINS,
        &|d| run_ok(d, &pin_args(MREC, "--replace", "src/a.rs=src/a.rs#L1-L3")),
        &|d| {
            y_edits_the_first_function_and_re_pins(d);
            bogus_pin(d, "code:src/a.rs@sha256:");
        },
    );
    refused_untouched(&dir, "does not hash to that side's pin");
}

#[test]
fn a_file_edited_after_its_re_pin_is_not_pinned_by_a_narrowing_merge_observed_failing() {
    let dir = moved(
        "moved-edited-after",
        TWO_PINS,
        &|d| run_ok(d, &pin_args(MREC, "--replace", "src/a.rs=src/a.rs#L1-L3")),
        &|d| {
            y_edits_the_first_function_and_re_pins(d);
            edit(d, "src/a.rs", 0, "8;");
        },
    );
    refused_untouched(&dir, "changed there without a re-pin");
}

#[test]
fn a_narrowing_over_a_pinned_file_still_conflicted_waits_for_it() {
    let dir = moved(
        "moved-unmerged-file",
        TWO_PINS,
        &|d| {
            edit(d, "src/a.rs", 0, "7;");
            run_ok(d, &pin_args(MREC, "--replace", "src/a.rs=src/a.rs#L1-L3"));
        },
        &y_edits_the_first_function_and_re_pins,
    );
    refused_untouched(&dir, "still unmerged");
}

#[test]
fn a_widening_the_other_side_re_pinned_the_old_range_of_is_refused_observed_failing() {
    let dir = moved(
        "moved-widened",
        RANGE_ITEM,
        &|d| run_ok(d, &pin_args(MREC, "--replace", "src/a.rs#L1-L3=src/a.rs")),
        &|d| {
            edit(d, "src/a.rs", 0, "9;");
            rebaseline(d);
        },
    );
    refused_untouched(&dir, "not the whole file");
}

#[test]
fn a_move_to_another_file_the_other_side_re_pinned_the_old_one_of_is_refused_observed_failing() {
    let dir = moved(
        "moved-other-file",
        ONE_FILE_ITEM,
        &|d| run_ok(d, &pin_args(MREC, "--replace", "src/a.rs=src/b.rs")),
        &y_edits_the_first_function_and_re_pins,
    );
    refused_untouched(&dir, "a different file");
}

#[test]
fn a_new_locator_is_not_paired_with_a_dropped_pin_of_another_file_observed_failing() {
    let dir = moved(
        "moved-wrong-path",
        TWO_FILE_RB,
        &|d| {
            run_ok(d, &pin_args(MREC, "--replace", "src/a.rs=src/a.rs#L1-L3"));
            run_ok(d, &pin_args(MREC, "--remove", "src/b.rs"));
        },
        &|d| {
            edit(d, "src/b.rs", 1, "3;");
            rebaseline(d);
            edit(d, "src/a.rs", 0, "9;");
        },
    );
    let v = mapply(&dir);
    assert!(
        v["conflicts"][0]["pins"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["recomputed"] == false),
        "{v}"
    );
    let text = std::fs::read_to_string(dir.join(MREC)).unwrap();
    let as_pinned_by_x = sha256("fn f() {\n    1;\n}");
    assert!(
        text.contains(&format!("src/a.rs#L1-L3@sha256:{as_pinned_by_x}")),
        "{text}"
    );
    assert!(!text.contains("replaced `src/b.rs`"), "{text}");
}

const TEN_LINES_ITEM: &str = "- A works. — evidence: code:src/a.rs#L1-L10@sha256:{l110}\nRB: code:src/a.rs#L1-L10@sha256:{l110}, test:t.rs\n";

fn rewrite(d: &Path, file: &str, f: &dyn Fn(&str) -> String) {
    let text = std::fs::read_to_string(d.join(file)).unwrap();
    std::fs::write(d.join(file), f(&text)).unwrap();
}

#[test]
fn a_range_whose_lines_moved_in_the_other_sides_file_is_not_narrowed_on_merge_observed_failing() {
    let dir = moved(
        "moved-range-shifted",
        TEN_LINES_ITEM,
        &|d| {
            run_ok(
                d,
                &pin_args(MREC, "--replace", "src/a.rs#L1-L10=src/a.rs#L1-L3"),
            )
        },
        &|d| {
            rewrite(d, "src/a.rs", &|t| {
                format!(
                    "// 1\n// 2\n// 3\n// 4\n// 5\n// 6\n// 7\n// 8\n// 9\n// 10\n{}",
                    t.replacen("    1;", "    9;", 1)
                )
            });
            rebaseline(d);
        },
    );
    refused_untouched(&dir, "not the whole file");
}

#[test]
fn a_range_the_other_side_edited_outside_of_is_not_narrowed_on_merge_observed_failing() {
    let dir = moved(
        "moved-range-outside",
        TEN_LINES_ITEM,
        &|d| {
            rewrite(d, "src/a.rs", &|t| {
                let lines: Vec<&str> = t.lines().collect();
                let kept: Vec<&str> = lines[..3].iter().chain(&lines[8..]).copied().collect();
                format!("{}\n", kept.join("\n"))
            });
            run_ok(
                d,
                &pin_args(MREC, "--replace", "src/a.rs#L1-L10=src/a.rs#L5-L6"),
            );
        },
        &|d| {
            rewrite(d, "src/a.rs", &|t| {
                let mut t = t.replacen("fn f() {", "fn f() { // y", 1);
                let last = t.rfind("}\n").unwrap();
                t.replace_range(last..last + 1, "} // y");
                t
            });
            rebaseline(d);
        },
    );
    refused_untouched(&dir, "not the whole file");
}

#[test]
fn two_narrowings_of_one_file_on_one_side_are_each_recomputed() {
    let dir = moved(
        "moved-two-sites",
        ONE_FILE_ITEM,
        &|d| {
            let a = std::fs::read_to_string(d.join("src/a.rs")).unwrap();
            let lines: Vec<&str> = a.lines().collect();
            let (h13, h911) = (
                sha256(&lines[..3].join("\n")),
                sha256(&lines[8..11].join("\n")),
            );
            let whole = format!("code:src/a.rs@sha256:{}", sha256(&a));
            rewrite(d, MREC, &|t| {
                t.replacen(&whole, &format!("code:src/a.rs#L1-L3@sha256:{h13}"), 1)
                    .replacen(&whole, &format!("code:src/a.rs#L9-L11@sha256:{h911}"), 1)
            });
        },
        &|d| {
            edit(d, "src/a.rs", 1, "8;");
            rebaseline(d);
        },
    );
    mapply(&dir);
    let text = std::fs::read_to_string(dir.join(MREC)).unwrap();
    let merged = std::fs::read_to_string(dir.join("src/a.rs")).unwrap();
    let lines: Vec<&str> = merged.lines().collect();
    for (scope, span) in [("L1-L3", &lines[..3]), ("L9-L11", &lines[8..11])] {
        let want = format!("code:src/a.rs#{scope}@sha256:{}", sha256(&span.join("\n")));
        assert_eq!(text.matches(&want).count(), 1, "{want}: {text}");
    }
}

const SPACERS_ITEM: &str = "- A works. — evidence: code:src/a.rs#L5-L7@sha256:{l57}\nRB: code:src/a.rs#L5-L7@sha256:{l57}, test:t.rs\n";

fn edit_text(d: &Path, from: &str, to: &str) {
    rewrite(d, "src/a.rs", &|t| t.replacen(from, to, 1));
}

fn mark_g_edited(d: &Path) {
    edit_text(d, "fn g() {", "fn g() { // EVIL");
}

fn drop_first_three_lines(d: &Path) {
    rewrite(d, "src/a.rs", &|t| {
        format!("{}\n", t.lines().skip(3).collect::<Vec<_>>().join("\n"))
    });
}

const OUTSIDE: &str = "changed code outside what that side pinned";

#[test]
fn a_narrowing_the_merge_shifts_onto_the_changers_unattested_edit_is_refused_observed_failing() {
    let dir = moved(
        "moved-evil",
        ONE_FILE_ITEM,
        &|d| {
            mark_g_edited(d);
            run_ok(d, &pin_args(MREC, "--replace", "src/a.rs=src/a.rs#L5-L7"));
        },
        &|d| {
            drop_first_three_lines(d);
            rebaseline(d);
        },
    );
    refused_untouched(&dir, OUTSIDE);
}

#[test]
fn a_narrowing_whose_changer_edited_outside_its_scope_is_refused_observed_failing() {
    let dir = moved(
        "moved-outside",
        ONE_FILE_ITEM,
        &|d| {
            mark_g_edited(d);
            run_ok(d, &pin_args(MREC, "--replace", "src/a.rs=src/a.rs#L5-L7"));
        },
        &y_edits_the_first_function_and_re_pins,
    );
    refused_untouched(&dir, OUTSIDE);
}

#[test]
fn a_range_the_merge_shifts_is_refused_observed_failing() {
    let dir = moved(
        "moved-shifted",
        ONE_FILE_ITEM,
        &|d| run_ok(d, &pin_args(MREC, "--replace", "src/a.rs=src/a.rs#L5-L7")),
        &|d| {
            drop_first_three_lines(d);
            rebaseline(d);
        },
    );
    refused_untouched(&dir, "edited or moved lines above it");
}

fn x_edits_the_first_spacer_and_g_and_re_pins(d: &Path) {
    edit_text(d, "fn spacer() {}", "fn spacer() { 1 }");
    mark_g_edited(d);
    rebaseline(d);
}

#[test]
fn two_range_re_pins_where_a_side_edited_outside_its_range_are_refused_observed_failing() {
    let dir = moved(
        "ordinary-outside",
        SPACERS_ITEM,
        &x_edits_the_first_spacer_and_g_and_re_pins,
        &|d| {
            edit_text(d, "fn spacer3() {}", "fn spacer3() { 3 }");
            rebaseline(d);
        },
    );
    refused_untouched(&dir, OUTSIDE);
}

#[test]
fn two_range_re_pins_the_merge_shifts_onto_an_unattested_edit_are_refused_observed_failing() {
    let dir = moved(
        "ordinary-evil",
        SPACERS_ITEM,
        &x_edits_the_first_spacer_and_g_and_re_pins,
        &|d| {
            drop_first_three_lines(d);
            edit_text(d, "fn spacer3() {}", "fn spacer3() { 3 }");
            rebaseline(d);
        },
    );
    refused_untouched(&dir, OUTSIDE);
}

fn lines_between(t: &str, from: usize, to: usize) -> String {
    t.lines()
        .skip(from - 1)
        .take(to - from + 1)
        .collect::<Vec<_>>()
        .join("\n")
}

fn delete_line(d: &Path, n: usize) {
    rewrite(d, "src/a.rs", &|t| {
        t.lines()
            .enumerate()
            .filter(|(i, _)| *i != n - 1)
            .map(|(_, l)| format!("{l}\n"))
            .collect()
    });
}

#[test]
fn both_sides_deleting_a_line_inside_a_range_are_refused_observed_failing() {
    let dir = moved(
        "both-delete",
        RANGE_ITEM,
        &|d| {
            delete_line(d, 1);
            rebaseline(d);
        },
        &|d| {
            delete_line(d, 3);
            rebaseline(d);
        },
    );
    refused_untouched(&dir, "does not end where");
}

const NESTED: &str = "mod outer {\nimpl X {\nfn f() {\n    1;\n    2;\n    3;\n}\nfn g() { 4; }\n}\nfn h() { 5; }\n}\n";
const BRACE_RESOLVER: &str = "locator_resolvers:\n  - name: rust\n    glob: \"**/*.rs\"\n    start: '^\\s*fn\\s+{symbol}\\b'\n    end: brace_balanced\n";

/// A symbol `f` whose two sides each change a different body line to a lone
/// opening brace, which the brace-blind resolver counts, so `f` ends later in
/// the merged file than in either side's.
fn symbol_that_grows_fixture() -> common::TempDir {
    let dir = common::TempDir::new("urzua-conflicts-symbol-grows");
    for d in ["docs/milestones", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    ok(&dir, &["init", "-q", "-b", "main"]);
    ok(&dir, &["config", "user.email", "test@example.com"]);
    ok(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{MILE_CONFIG}{BRACE_RESOLVER}"),
    )
    .unwrap();
    std::fs::write(dir.join("t.rs"), "t\n").unwrap();
    std::fs::write(dir.join("src/a.rs"), NESTED).unwrap();
    std::fs::write(dir.join("src/b.rs"), B_RS).unwrap();
    let h = sha256(&lines_between(NESTED, 3, 9));
    let items = format!("- A works. — evidence: code:src/a.rs#f@sha256:{h}\nRB: code:src/a.rs#f@sha256:{h}, test:t.rs\n");
    std::fs::write(dir.join(MREC), milestone(1, &dir, &items)).unwrap();
    commit_all(&dir, "base");
    for (branch, from) in [("x", "    1;"), ("y", "    3;")] {
        ok(&dir, &["checkout", "-q", "main"]);
        ok(&dir, &["checkout", "-q", "-b", branch]);
        edit_text(&dir, from, "    \"{\";");
        rebaseline(&dir);
        commit_all(&dir, branch);
    }
    ok(&dir, &["checkout", "-q", "main"]);
    ok(&dir, &["merge", "-q", "--ff-only", "x"]);
    ok(&dir, &["checkout", "-q", "y"]);
    assert!(!git(&dir, &["merge", "-q", "main"]), "the merge conflicts");
    dir
}

#[test]
fn a_symbol_whose_span_grows_past_both_sides_is_refused_observed_failing() {
    let dir = symbol_that_grows_fixture();
    refused_untouched(&dir, "does not end where");
}

const TWO_A_ITEM: &str =
    "- A works. — evidence: code:src/a.rs@sha256:{a}, code:src/a.rs#L5-L7@sha256:{l57}\n";

fn narrow_two_ways(name: &str, far_first: bool) -> common::TempDir {
    moved(
        name,
        TWO_A_ITEM,
        &move |d| {
            let a = std::fs::read_to_string(d.join("src/a.rs")).unwrap();
            let n13 = format!(
                "code:src/a.rs#L1-L3@sha256:{}",
                sha256(&lines_between(&a, 1, 3))
            );
            let n911 = format!(
                "code:src/a.rs#L9-L11@sha256:{}",
                sha256(&lines_between(&a, 9, 11))
            );
            let new = if far_first {
                format!("{n13}, {n911}")
            } else {
                format!("{n911}, {n13}")
            };
            let old = format!(
                "code:src/a.rs@sha256:{}, code:src/a.rs#L5-L7@sha256:{}",
                sha256(&a),
                sha256(&lines_between(&a, 5, 7))
            );
            rewrite(d, MREC, &|t| t.replacen(&old, &new, 1));
        },
        &|d| {
            edit(d, "src/a.rs", 1, "3;");
            rebaseline(d);
        },
    )
}

#[test]
fn several_dropped_locators_of_one_file_are_refused_whatever_the_order_written_observed_failing() {
    for (name, far_first) in [("order-far", true), ("order-near", false)] {
        let dir = narrow_two_ways(name, far_first);
        refused_untouched(&dir, "could replace more than one dropped locator");
    }
}

const B_FIRST_ITEM: &str = "- A works. — evidence: code:src/a.rs@sha256:{a}\nRB: code:src/b.rs@sha256:{b}, code:src/a.rs@sha256:{a}, test:t.rs\n";

#[test]
fn two_files_narrowed_in_one_site_in_another_order_are_each_recomputed() {
    let dir = moved(
        "two-files",
        B_FIRST_ITEM,
        &|d| {
            let a = std::fs::read_to_string(d.join("src/a.rs")).unwrap();
            let b = std::fs::read_to_string(d.join("src/b.rs")).unwrap();
            let na = format!(
                "code:src/a.rs#L1-L3@sha256:{}",
                sha256(&lines_between(&a, 1, 3))
            );
            let nb = format!(
                "code:src/b.rs#L9-L11@sha256:{}",
                sha256(&lines_between(&b, 9, 11))
            );
            let old = format!(
                "code:src/b.rs@sha256:{}, code:src/a.rs@sha256:{}",
                sha256(&b),
                sha256(&a)
            );
            rewrite(d, MREC, &|t| t.replacen(&old, &format!("{na}, {nb}"), 1));
        },
        &|d| {
            edit(d, "src/a.rs", 0, "9;");
            edit(d, "src/b.rs", 1, "8;");
            rebaseline(d);
        },
    );
    mapply(&dir);
    let text = std::fs::read_to_string(dir.join(MREC)).unwrap();
    for (file, span) in [("a", (1, 3)), ("b", (9, 11))] {
        let merged = std::fs::read_to_string(dir.join(format!("src/{file}.rs"))).unwrap();
        let want = format!(
            "src/{file}.rs#L{}-L{}@sha256:{}",
            span.0,
            span.1,
            sha256(&lines_between(&merged, span.0, span.1))
        );
        assert!(text.contains(&want), "{want}: {text}");
    }
}

fn symbol_config() -> String {
    format!("{MILE_CONFIG}{BRACE_RESOLVER}")
}

fn narrow_to_g_args() -> Vec<&'static str> {
    vec![
        "fix",
        "--pin",
        "--apply",
        "--ids",
        MREC,
        "--replace",
        "src/a.rs=src/a.rs#g",
        "--by",
        "tester",
        "--actor",
        "agent",
        "--reason",
        "the claim is about g only",
    ]
}

fn symbol_narrowing(name: &str, other_side_edit_line: usize) -> common::TempDir {
    let dir = mfixture_with(
        name,
        &symbol_config(),
        &[(MREC.to_string(), ONE_FILE_ITEM.to_string())],
        &|d| run_ok(d, &narrow_to_g_args()),
        &move |d| {
            edit(d, "src/a.rs", other_side_edit_line, "9;");
            rebaseline(d);
        },
    );
    assert!(!git(&dir, &["merge", "-q", "main"]), "the merge conflicts");
    dir
}

#[test]
fn a_whole_file_narrowed_to_a_symbol_resolves_when_the_other_side_edited_above_it_observed_failing()
{
    let dir = symbol_narrowing("symbol-above", 0);
    mapply(&dir);
    let text = std::fs::read_to_string(dir.join(MREC)).unwrap();
    let merged = std::fs::read_to_string(dir.join("src/a.rs")).unwrap();
    let want = format!(
        "src/a.rs#g@sha256:{}",
        sha256(&lines_between(&merged, 9, 11))
    );
    assert_eq!(text.matches(&want).count(), 2, "{text}");
    assert!(!text.contains("code:src/a.rs@sha256:"), "{text}");
}

#[test]
fn a_range_with_edited_lines_above_it_is_refused_saying_the_other_side_edited_above_observed_failing(
) {
    let dir = moved(
        "range-above",
        ONE_FILE_ITEM,
        &|d| run_ok(d, &pin_args(MREC, "--replace", "src/a.rs=src/a.rs#L9-L11")),
        &y_edits_the_first_function_and_re_pins,
    );
    refused_untouched(&dir, "edited or moved lines above it");
}

#[test]
fn one_dropped_locator_two_new_ones_replace_is_refused_observed_failing() {
    let dir = moved(
        "split-whole-file",
        ONE_FILE_ITEM,
        &|d| {
            let a = std::fs::read_to_string(d.join("src/a.rs")).unwrap();
            let n13 = format!(
                "code:src/a.rs#L1-L3@sha256:{}",
                sha256(&lines_between(&a, 1, 3))
            );
            let n911 = format!(
                "code:src/a.rs#L9-L11@sha256:{}",
                sha256(&lines_between(&a, 9, 11))
            );
            let old = format!("code:src/a.rs@sha256:{}", sha256(&a));
            rewrite(d, MREC, &|t| t.replacen(&old, &format!("{n13}, {n911}"), 1));
        },
        &|d| {
            edit(d, "src/a.rs", 1, "3;");
            rebaseline(d);
        },
    );
    refused_untouched(&dir, "share one with another new locator");
}
