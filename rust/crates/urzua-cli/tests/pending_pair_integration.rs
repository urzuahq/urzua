//! A paired change to two records without the shell (`MILE-287`): the editor
//! hook lets the first half of a reciprocal relation through and names the
//! other half (`harness.guardrails.editor-hook.pending_pair`, default
//! `allow`), or refuses it naming `urzua fix` (`refuse`); `urzua fix` writes a
//! missing inverse. Real git repositories and the real binary.

mod common;

use std::io::Write;
use std::path::Path;
use std::process::Stdio;

const A: &str = "docs/adr/ADR-1-new.md";
const B: &str = "docs/adr/ADR-2-old.md";
const LOG: &str = "\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-08-20 | Initial. | **structural** |\n";

fn config(pending_pair: &str) -> String {
    format!(
        "schema_version: 2
rules:
  header.required-fields: error
  relation.pair-reciprocity: error
record_types:
  adr:
    dir: \"docs/adr\"
    required_fields: [\"Status\"]
    known_fields: [\"Supersedes\", \"Superseded-by\"]
    statuses:
      values: [Proposed, Accepted, Superseded]
  note:
    dir: \"docs/notes\"
    required_fields: [\"Status\"]
    known_fields: [\"Supersedes\", \"Superseded-by\"]
relation_pairs:
  - name: supersession
    forward: \"Supersedes\"
    inverse: \"Superseded-by\"
    reciprocity: required
    types: [adr]
harness:
  guardrails:
    acceptance: {{enabled: true}}
    protected-paths: {{enabled: true}}
    editor-hook: {{enabled: true{pending_pair}}}
authority:
  deciders:
    maintainers: {{logins: [Alice]}}
  gates:
    - {{type: adr, to: [Accepted, Superseded], by: maintainers}}
    - {{paths: [\".urzua/config.yaml\"], by: maintainers}}
"
    )
}

fn record(n: u32, fields: &str) -> String {
    format!("---\nStatus: Proposed\n{fields}---\n# ADR-{n}: T\n\nBody.\n{LOG}")
}

fn project(name: &str, pending_pair: &str, files: &[(&str, String)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-pending-{name}"));
    for d in ["docs/adr", "docs/notes", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    common::git(&dir, &["init", "-q"]);
    common::git(&dir, &["config", "user.email", "t@example.com"]);
    common::git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config(pending_pair)).unwrap();
    for (path, text) in files {
        std::fs::write(dir.join(path), text).unwrap();
    }
    common::git(&dir, &["add", "-A"]);
    common::git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn pair_project(name: &str, pending_pair: &str) -> common::TempDir {
    project(
        name,
        pending_pair,
        &[(A, record(1, "")), (B, record(2, ""))],
    )
}

/// One agent edit through the editor hook, as the translator sends it.
fn edit(dir: &Path, rel: &str, content: &str) -> (i32, serde_json::Value) {
    let request = serde_json::json!({
        "event": "edit", "tool": "Write", "paths": [dir.join(rel).to_string_lossy()],
        "content": content, "actor": "agent",
        "cwd": dir.to_string_lossy(), "project_dir": dir.to_string_lossy(),
    });
    let mut child = common::urzua()
        .args(["harness", "run", "edit"])
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {out:?}"));
    (out.status.code().unwrap(), v)
}

fn run(dir: &Path, args: &[&str]) -> (i32, serde_json::Value) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    let v =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {out:?}"));
    (out.status.code().unwrap(), v)
}

fn message(v: &serde_json::Value) -> &str {
    v["message"].as_str().unwrap()
}

fn rules(v: &serde_json::Value) -> Vec<String> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn refuse_blocks_each_half_and_names_the_fix_command_that_makes_check_clean() {
    let dir = pair_project("refuse", ", pending_pair: refuse");
    let half_a = record(1, "Supersedes: ADR-2\n");
    let (code, v) = edit(&dir, A, &half_a);
    assert_eq!((code, v["verdict"].as_str().unwrap()), (1, "block"), "{v}");
    assert_eq!(rules(&v), ["relation.pair-reciprocity"], "{v}");
    let fix = format!("urzua fix --apply --ids {B}");
    assert!(message(&v).contains(&fix), "{v}");
    assert!(
        message(&v).contains("add ADR-1 to Superseded-by in docs/adr/ADR-2-old.md"),
        "{v}"
    );

    // The mirror half is refused for the mirror reason.
    let (code, v) = edit(&dir, B, &record(2, "Superseded-by: ADR-1\n"));
    assert_eq!(code, 1, "{v}");
    assert!(
        message(&v).contains(&format!("urzua fix --apply --ids {A}")),
        "{v}"
    );

    // Half A written (outside the hook), then the command as printed.
    std::fs::write(dir.join(A), &half_a).unwrap();
    let (code, check) = run(&dir, &["check"]);
    assert_eq!(code, 1, "{check}");
    assert_eq!(check["next_action"].as_str().unwrap(), fix, "{check}");
    let argv: Vec<&str> = fix.split(' ').skip(1).collect();
    let (code, applied) = run(&dir, &argv);
    assert_eq!(code, 0, "{applied}");
    assert_eq!(applied["status"], "applied", "{applied}");
    let b = std::fs::read_to_string(dir.join(B)).unwrap();
    assert!(b.contains("\nSuperseded-by: ADR-1\n"), "{b}");
    assert!(b.contains("Status: Proposed"), "{b}");
    assert!(b.contains("the inverse that"), "{b}");
    let (code, check) = run(&dir, &["check"]);
    assert_eq!(
        (code, check["status"].as_str().unwrap()),
        (0, "ok"),
        "{check}"
    );
}

#[test]
fn by_default_the_first_half_is_allowed_with_the_follow_up_and_the_second_completes_it() {
    let dir = pair_project("allow", "");
    let half_a = record(1, "Supersedes: ADR-2\n");
    let (code, v) = edit(&dir, A, &half_a);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["verdict"], "allow-with-warnings", "{v}");
    let m = message(&v);
    assert!(m.contains("Now required"), "{m}");
    assert!(
        m.contains("add ADR-1 to Superseded-by in docs/adr/ADR-2-old.md"),
        "{m}"
    );
    assert!(m.contains(&format!("urzua fix --apply --ids {B}")), "{m}");
    assert!(m.contains("pre-commit and CI block it"), "{m}");
    std::fs::write(dir.join(A), &half_a).unwrap();

    // The second half now matches, so nothing is introduced.
    let half_b = record(2, "Superseded-by: ADR-1\n");
    let (code, v) = edit(&dir, B, &half_b);
    assert_eq!((code, v["verdict"].as_str().unwrap()), (0, "allow"), "{v}");
    std::fs::write(dir.join(B), &half_b).unwrap();
    let (code, check) = run(&dir, &["check"]);
    assert_eq!(code, 0, "{check}");
}

#[test]
fn an_unmatched_half_left_in_the_tree_still_fails_check() {
    let dir = pair_project("left", "");
    std::fs::write(dir.join(A), record(1, "Supersedes: ADR-2\n")).unwrap();
    let (code, check) = run(&dir, &["check"]);
    assert_eq!(code, 1, "{check}");
}

#[test]
fn an_unrelated_error_in_the_same_edit_still_refuses_and_names_the_pair_follow_up() {
    let dir = pair_project("unrelated", "");
    let broken = "---\nSupersedes: ADR-2\n---\n# ADR-1: T\n\nBody.\n";
    let (code, v) = edit(&dir, A, broken);
    assert_eq!((code, v["verdict"].as_str().unwrap()), (1, "block"), "{v}");
    let r = rules(&v);
    assert!(r.contains(&"header.required-fields".to_string()), "{v}");
    assert!(
        message(&v).contains(&format!("urzua fix --apply --ids {B}")),
        "{v}"
    );
}

#[test]
fn a_gated_status_move_with_the_second_half_is_still_refused() {
    let dir = pair_project("gated", "");
    std::fs::write(dir.join(A), record(1, "Supersedes: ADR-2\n")).unwrap();
    let moved =
        record(2, "Superseded-by: ADR-1\n").replace("Status: Proposed", "Status: Superseded");
    let (code, v) = edit(&dir, B, &moved);
    assert_eq!((code, v["verdict"].as_str().unwrap()), (1, "block"), "{v}");
    assert!(message(&v).contains("refused"), "{v}");
}

#[test]
fn an_unknown_pending_pair_value_stops_the_load() {
    let dir = pair_project("bad", ", pending_pair: maybe");
    let (code, v) = run(&dir, &["check"]);
    assert_eq!(code, 2, "{v}");
    assert!(
        v.to_string()
            .contains("pending_pair must be refuse or allow"),
        "{v}"
    );
}

fn repairs(v: &serde_json::Value) -> Vec<(String, String, String)> {
    v["repairs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["record"].as_str().unwrap().to_string(),
                r["field"].as_str().unwrap().to_string(),
                r["computed_value"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[test]
fn fix_lists_inverses_for_a_list_valued_claim_and_appends_to_an_existing_list() {
    let dir = project(
        "lists",
        "",
        &[
            (A, record(1, "Supersedes: ADR-2, ADR-3\n")),
            (B, record(2, "")),
            ("docs/adr/ADR-3-c.md", record(3, "Superseded-by: ADR-5\n")),
            ("docs/adr/ADR-5-e.md", record(5, "Supersedes: ADR-3\n")),
        ],
    );
    let (_, v) = run(&dir, &["fix"]);
    assert_eq!(v["status"], "repairs-available", "{v}");
    let mut got = repairs(&v);
    got.sort();
    assert_eq!(
        got,
        [
            (
                B.to_string(),
                "Superseded-by".to_string(),
                "ADR-1".to_string()
            ),
            (
                "docs/adr/ADR-3-c.md".to_string(),
                "Superseded-by".to_string(),
                "ADR-5, ADR-1".to_string()
            ),
        ],
        "{v}"
    );
    let status_note = v["repairs"][0]["inverse"]["status"].as_str().unwrap();
    assert!(status_note.contains("separate step"), "{v}");
    let (code, v) = run(
        &dir,
        &["fix", "--apply", "--force", "--by", "t", "--actor", "agent"],
    );
    assert_eq!(code, 0, "{v}");
    let c = std::fs::read_to_string(dir.join("docs/adr/ADR-3-c.md")).unwrap();
    assert!(c.contains("\nSuperseded-by: ADR-5, ADR-1\n"), "{c}");
    let (code, check) = run(&dir, &["check"]);
    assert_eq!(code, 0, "{check}");
}

#[test]
fn fix_writes_the_forward_side_when_only_the_inverse_was_written() {
    let dir = project(
        "reverse",
        "",
        &[(A, record(1, "")), (B, record(2, "Superseded-by: ADR-1\n"))],
    );
    let (_, v) = run(&dir, &["fix"]);
    assert_eq!(
        repairs(&v),
        [(A.to_string(), "Supersedes".to_string(), "ADR-2".to_string())],
        "{v}"
    );
    let (code, v) = run(
        &dir,
        &[
            "fix", "--apply", "--ids", A, "--by", "t", "--actor", "agent",
        ],
    );
    assert_eq!(code, 0, "{v}");
    let a = std::fs::read_to_string(dir.join(A)).unwrap();
    assert!(
        a.starts_with("---\nStatus: Proposed\nSupersedes: ADR-2\n---\n"),
        "{a}"
    );
    let (code, check) = run(&dir, &["check"]);
    assert_eq!(code, 0, "{check}");
}

#[test]
fn fix_never_overwrites_a_different_value_and_reports_the_conflict() {
    let dir = project(
        "conflict",
        "",
        &[
            (A, record(1, "Supersedes: ADR-2\n")),
            (B, record(2, "Superseded-by: under review\n")),
        ],
    );
    let (_, v) = run(&dir, &["fix"]);
    assert!(repairs(&v).is_empty(), "{v}");
    assert!(
        v["notices"].to_string().contains("is not overwritten"),
        "{v}"
    );
}

#[test]
fn fix_writes_nothing_for_a_target_that_does_not_exist_and_says_so() {
    let dir = project("missing", "", &[(A, record(1, "Supersedes: ADR-9\n"))]);
    let (_, v) = run(&dir, &["fix"]);
    assert!(repairs(&v).is_empty(), "{v}");
    assert!(v["notices"].to_string().contains("does not exist"), "{v}");
}

#[test]
fn fix_writes_nothing_to_a_type_the_pair_is_not_declared_for() {
    let dir = project(
        "scoped",
        "",
        &[
            (A, record(1, "Supersedes: NOTE-4\n")),
            (
                "docs/notes/NOTE-4-n.md",
                record(4, "").replace("# ADR-4", "# NOTE-4"),
            ),
        ],
    );
    let (_, v) = run(&dir, &["fix"]);
    assert!(repairs(&v).is_empty(), "{v}");
}
