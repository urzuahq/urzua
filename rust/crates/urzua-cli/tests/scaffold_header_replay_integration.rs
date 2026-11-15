//! MILE-103: `scripts/replay-header-comparator.py` replays `header.scaffold-mismatch`
//! over the pull requests merged into a main branch and reports which records
//! differed from the scaffold at some head, which of those a header rule also
//! rejected, and which merged records still differ. Real git fixtures (a
//! repository with `refs/pull/<n>/head` the way a host publishes them), the real
//! binary, the real script.
//!
//! Verifies: SPEC-2, SPEC-12

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str], date: Option<&str>) -> String {
    let mut command = Command::new("git");
    command
        .args(["-c", "user.email=t@example.com", "-c", "user.name=t"])
        .args(args)
        .current_dir(dir);
    if let Some(date) = date {
        command
            .env("GIT_AUTHOR_DATE", date)
            .env("GIT_COMMITTER_DATE", date);
    }
    let out = command.output().expect("git should run");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn write(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// Every `bug` record declares `Status` and `Author`; with no declared initial
/// status `urzua new` writes `Status` null, so the fields are the whole scaffold.
const CONFIG: &str = "schema_version: 2\nrules:\n  header.required-fields: error\nrecord_types:\n  bug:\n    dir: docs/bug\n    prefix: BUG\n    required_fields: [Status, Author]\n    header_shape: yaml-frontmatter\n";

fn bug(n: u32, id: &str, author: Option<&str>) -> String {
    let author = author.map_or(String::new(), |a| format!("Author: {a}\n"));
    format!("---\nStable-Id: {id}\nStatus: Open\n{author}---\n# BUG-{n} - x\n")
}

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scripts/replay-header-comparator.py")
}

/// `main` with a root commit (long ago) holding BUG-1, then:
/// - pull request 1 adds BUG-2 exactly as `urzua new` writes it;
/// - pull request 2 adds BUG-3 by hand without `Author` (a header rule rejects
///   it, and it differs from the scaffold), then a second commit adds the
///   `Author`, so the merged record is the scaffold's.
///
/// Each merges into `main` as a squash commit ending `(#n)`.
fn world(name: &str) -> common::TempDir {
    let root = common::TempDir::new(&format!("urzua-replay-{name}"));
    let dir: &Path = &root;
    git(dir, &["init", "-q", "-b", "main"], None);
    write(dir, ".urzua/config.yaml", CONFIG);
    write(
        dir,
        "docs/bug/BUG-1-a.md",
        &bug(1, "01M9REPLAY00000000000000A1", Some("ann")),
    );
    git(dir, &["add", "-A"], None);
    git(
        dir,
        &["commit", "-q", "-m", "root"],
        Some("2020-01-01T12:00:00Z"),
    );

    git(dir, &["checkout", "-q", "-b", "pr1"], None);
    write(
        dir,
        "docs/bug/BUG-2-b.md",
        &bug(2, "01M9REPLAY00000000000000B2", Some("bo")),
    );
    git(dir, &["add", "-A"], None);
    git(dir, &["commit", "-q", "-m", "add BUG-2"], None);
    git(dir, &["update-ref", "refs/pull/1/head", "HEAD"], None);
    git(dir, &["checkout", "-q", "main"], None);
    git(dir, &["merge", "-q", "--squash", "pr1"], None);
    git(dir, &["commit", "-q", "-m", "Add BUG-2 (#1)"], None);

    git(dir, &["checkout", "-q", "-b", "pr2"], None);
    write(
        dir,
        "docs/bug/BUG-3-c.md",
        &bug(3, "01M9REPLAY00000000000000C3", None),
    );
    git(dir, &["add", "-A"], None);
    git(dir, &["commit", "-q", "-m", "add BUG-3 by hand"], None);
    write(
        dir,
        "docs/bug/BUG-3-c.md",
        &bug(3, "01M9REPLAY00000000000000C3", Some("cy")),
    );
    git(dir, &["add", "-A"], None);
    git(dir, &["commit", "-q", "-m", "fix BUG-3"], None);
    git(dir, &["update-ref", "refs/pull/2/head", "HEAD"], None);
    git(dir, &["checkout", "-q", "main"], None);
    git(dir, &["merge", "-q", "--squash", "pr2"], None);
    git(dir, &["commit", "-q", "-m", "Fix BUG-3 (MILE-9, #2)"], None);
    root
}

fn replay(dir: &Path, extra: &[&str]) -> serde_json::Value {
    let out = Command::new("python3")
        .arg(script())
        .args(["--repo"])
        .arg(dir)
        .args(["--bin", env!("CARGO_BIN_EXE_urzua")])
        .args(["--main-ref", "main", "--since", "2021-01-01"])
        .args(extra)
        .output()
        .expect("python3 should run");
    assert!(
        out.status.success(),
        "the replay failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("the replay prints JSON on stdout")
}

#[test]
fn a_record_a_header_rule_rejected_and_the_scaffold_would_have_written_differently_is_counted() {
    let root = world("rejected");
    let report = replay(&root, &[]);
    assert_eq!(report["merged_pull_requests"], 2, "{report}");
    // BUG-3 differed at pull request 2's first commit, and the same commit had
    // `header.required-fields` reporting it: rejected. BUG-2 never differed.
    assert_eq!(report["pairs_that_differed_at_any_head"], 1, "{report}");
    assert_eq!(report["pairs_rejected"], 1, "{report}");
    let rejected = &report["rejected"][0];
    assert_eq!(rejected["pr"], 2);
    assert_eq!(rejected["file"], "docs/bug/BUG-3-c.md");
    assert_eq!(
        rejected["header_rules"],
        serde_json::json!(["header.required-fields"])
    );
    assert!(
        rejected["differences"]
            .as_str()
            .unwrap()
            .contains("'Author' is missing"),
        "{rejected}"
    );
    // The fix commit is the pull request's final head and conforms.
    assert_eq!(report["pairs_differing_at_final_head"], 0, "{report}");
    // Both merged records are examined and both match the scaffold.
    assert_eq!(report["merged_records_examined"], 2, "{report}");
    assert_eq!(
        report["merged_records_differing_from_scaffold"], 0,
        "{report}"
    );
    assert_eq!(report["commits_that_could_not_run"], serde_json::json!([]));
}

#[test]
fn a_difference_no_header_rule_reported_is_listed_but_not_rejected() {
    let root = world("not-rejected");
    // Pull request 3 adds BUG-4 with a field the scaffold does not write; every
    // header rule is satisfied, so it differs without being rejected.
    let dir: &Path = &root;
    git(dir, &["checkout", "-q", "-b", "pr3"], None);
    write(
        dir,
        "docs/bug/BUG-4-d.md",
        "---\nStable-Id: 01M9REPLAY00000000000000D4\nStatus: Open\nAuthor: di\nTags: x\n---\n# BUG-4 - x\n",
    );
    git(dir, &["add", "-A"], None);
    git(dir, &["commit", "-q", "-m", "add BUG-4"], None);
    git(dir, &["update-ref", "refs/pull/3/head", "HEAD"], None);
    git(dir, &["checkout", "-q", "main"], None);
    git(dir, &["merge", "-q", "--squash", "pr3"], None);
    git(dir, &["commit", "-q", "-m", "Add BUG-4 (#3)"], None);
    let report = replay(dir, &[]);
    assert_eq!(
        report["pairs_rejected"], 1,
        "BUG-3 is still rejected: {report}"
    );
    assert_eq!(report["pairs_that_differed_at_any_head"], 2, "{report}");
    assert_eq!(report["pairs_differing_at_final_head"], 1, "{report}");
    assert_eq!(
        report["merged_records_differing_from_scaffold"], 1,
        "{report}"
    );
    assert_eq!(report["merged_differing"][0]["file"], "docs/bug/BUG-4-d.md");
    assert_eq!(
        report["differed_but_not_rejected"][0]["file"],
        "docs/bug/BUG-4-d.md"
    );
}

#[test]
fn the_replay_leaves_the_checkout_it_runs_from_untouched() {
    let root = world("untouched");
    let before = git(&root, &["status", "--porcelain"], None);
    let head = git(&root, &["rev-parse", "HEAD"], None);
    let _ = replay(&root, &[]);
    assert_eq!(git(&root, &["status", "--porcelain"], None), before);
    assert_eq!(git(&root, &["rev-parse", "HEAD"], None), head);
    assert_eq!(
        git(&root, &["worktree", "list", "--porcelain"], None)
            .matches("worktree ")
            .count(),
        1,
        "the throwaway worktrees are removed"
    );
}
