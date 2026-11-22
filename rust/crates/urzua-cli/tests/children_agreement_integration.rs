//! `status.children-agreement`: a record's status against the records that are
//! part of it, driven end to end through the built binary: the finding on the
//! parent, the suggested action, a `--base` run where a child's change causes
//! it, the configuration that cannot support it, a repository that did not opt
//! in, and the post-command report that names the parent when the command
//! changed a child.

mod common;

use serde_json::Value;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;

const RULE: &str = "status.children-agreement";

const STATUSES: &str = "    statuses:
      values: [Planned, Active, Done, Dropped]
      initial: [Planned]
      progress: [Active]
      success: [Done]
      failure: [Dropped]
      transitions:
        Planned: [Active, Dropped]
        Active: [Done, Dropped]
        Done: [Active]
";

fn config(rule: &str, statuses: &str, extra: &str) -> String {
    format!(
        "schema_version: 2
{rule}record_types:
  work:
    dir: docs/work
    prefix: WORK
    header_shape: yaml-frontmatter
    required_fields: [Status]
    known_fields: [Part-of]
    narrative_fields: []
    pointer_fields:
      - {{name: Part-of, targets: [work], part_of: true}}
{statuses}{extra}"
    )
}

fn on() -> String {
    config("rules:\n  status.children-agreement: error\n", STATUSES, "")
}

fn record(n: u32, status: &str, part_of: &str) -> String {
    let line = if part_of.is_empty() {
        String::new()
    } else {
        format!("Part-of: {part_of}\n")
    };
    format!("---\nStatus: {status}\n{line}---\n# {n} - Work\n")
}

fn path(n: u32) -> String {
    format!("docs/work/WORK-{n}-w.md")
}

fn repo(name: &str, config: &str, records: &[(u32, &str, &str)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-children-{name}"));
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/work")).unwrap();
    std::fs::write(dir.join(".gitignore"), ".urzua/cache/\n").unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    for (n, status, part_of) in records {
        std::fs::write(dir.join(path(*n)), record(*n, status, part_of)).unwrap();
    }
    common::git(&dir, &["init", "-q"]);
    common::commit_all(&dir, 0);
    dir
}

fn set(dir: &Path, n: u32, status: &str, part_of: &str) {
    std::fs::write(dir.join(path(n)), record(n, status, part_of)).unwrap();
}

fn run(dir: &Path, args: &[&str]) -> (Option<i32>, String, Value) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let v = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}: {text}"));
    (out.status.code(), text, v)
}

fn findings(v: &Value) -> Vec<&Value> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == RULE)
        .collect()
}

fn notices(v: &Value) -> Vec<&Value> {
    v["notices"]
        .as_array()
        .map(|n| n.iter().filter(|n| n["subject"] == RULE).collect())
        .unwrap_or_default()
}

#[test]
fn a_parent_not_started_while_a_child_is_under_way_is_an_error_on_the_parent_with_the_move_offered()
{
    let dir = repo(
        "start",
        &on(),
        &[
            (1, "Planned", ""),
            (2, "Active", "WORK-1"),
            (3, "Done", "WORK-1"),
        ],
    );
    let (code, _, v) = run(&dir, &["check"]);
    assert_eq!(code, Some(1), "{v}");
    let found = findings(&v);
    assert_eq!(found.len(), 1, "{v}");
    assert_eq!(found[0]["severity"], "error");
    assert_eq!(found[0]["file"], path(1));
    let m = found[0]["message"].as_str().unwrap();
    assert!(
        m.contains("WORK-1")
            && m.contains("'Planned'")
            && m.contains("1 finished, 1 under-way")
            && m.contains("WORK-2, WORK-3"),
        "{m}"
    );
    let action = &found[0]["suggested_action"];
    assert_eq!(action["kind"], "edit-field");
    assert_eq!(action["op"], "set");
    assert_eq!(action["target"]["field"], "Status");
    assert_eq!(action["target"]["file"], path(1));
    assert_eq!(action["value"], "Active");
}

#[test]
fn a_finished_parent_with_an_open_child_is_an_error_with_no_action() {
    let dir = repo(
        "finished",
        &on(),
        &[(1, "Done", ""), (2, "Planned", "WORK-1")],
    );
    let (_, _, v) = run(&dir, &["check"]);
    let found = findings(&v);
    assert_eq!(found.len(), 1, "{v}");
    assert_eq!(found[0]["file"], path(1));
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("which finishes it"));
    assert!(found[0]["suggested_action"].is_null(), "{v}");
}

#[test]
fn the_cases_the_rule_leaves_alone_report_nothing() {
    for records in [
        vec![
            (1, "Active", ""),
            (2, "Done", "WORK-1"),
            (3, "Done", "WORK-1"),
        ],
        vec![(1, "Dropped", ""), (2, "Active", "WORK-1")],
        vec![
            (1, "Done", ""),
            (2, "Done", "WORK-1"),
            (3, "Dropped", "WORK-1"),
        ],
        vec![(1, "Planned", ""), (2, "Planned", "WORK-1")],
    ] {
        let dir = repo("quiet", &on(), &records);
        let (code, _, v) = run(&dir, &["check"]);
        assert!(findings(&v).is_empty(), "{records:?}: {v}");
        assert_eq!(code, Some(0), "{records:?}: {v}");
    }
}

#[test]
fn a_gated_start_move_is_not_offered_and_the_finding_says_who_may_make_it() {
    let gated = config(
        "rules:\n  status.children-agreement: error\n",
        STATUSES,
        "authority:\n  deciders:\n    leads: {logins: [someone]}\n  gates:\n    - {type: work, to: [Active], by: leads}\n",
    );
    let dir = repo(
        "gated",
        &gated,
        &[(1, "Planned", ""), (2, "Active", "WORK-1")],
    );
    let (_, _, v) = run(&dir, &["check"]);
    let found = findings(&v);
    assert_eq!(found.len(), 1, "{v}");
    assert!(found[0]["suggested_action"].is_null(), "{v}");
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("reserved for leads"));
}

#[test]
fn fix_does_not_write_the_parent_status() {
    let dir = repo("fix", &on(), &[(1, "Planned", ""), (2, "Active", "WORK-1")]);
    let before = std::fs::read_to_string(dir.join(path(1))).unwrap();
    for args in [&["fix"][..], &["fix", "--apply", "--actor", "agent"][..]] {
        let _ = common::urzua()
            .args(args)
            .current_dir(&dir)
            .output()
            .unwrap();
        assert_eq!(std::fs::read_to_string(dir.join(path(1))).unwrap(), before);
    }
}

#[test]
fn a_change_to_a_child_that_creates_the_disagreement_is_reported_on_the_parent_under_base() {
    let dir = repo(
        "base",
        &on(),
        &[(1, "Planned", ""), (2, "Planned", "WORK-1")],
    );
    set(&dir, 2, "Active", "WORK-1");
    let (_, _, v) = run(&dir, &["check", "--base", "HEAD"]);
    let found = findings(&v);
    assert_eq!(found.len(), 1, "{v}");
    assert_eq!(found[0]["file"], path(1));
    assert_eq!(
        v["scope"]["dependents"],
        serde_json::json!([path(1)]),
        "{v}"
    );
}

#[test]
fn a_new_child_under_a_started_parent_is_reported_under_base_and_an_unrelated_change_is_not() {
    let dir = repo("base-new", &on(), &[(1, "Done", ""), (2, "Done", "WORK-1")]);
    set(&dir, 3, "Active", "WORK-1");
    common::git(&dir, &["add", "-A"]);
    let (_, _, v) = run(&dir, &["check", "--base", "HEAD"]);
    assert_eq!(findings(&v).len(), 1, "{v}");
    assert_eq!(findings(&v)[0]["file"], path(1));

    let quiet = repo(
        "base-body",
        &on(),
        &[
            (1, "Planned", ""),
            (2, "Planned", "WORK-1"),
            (3, "Active", ""),
        ],
    );
    std::fs::write(
        quiet.join(path(2)),
        format!("{}\nMore prose.\n", record(2, "Planned", "WORK-1")),
    )
    .unwrap();
    let (_, _, v) = run(&quiet, &["check", "--base", "HEAD"]);
    assert!(findings(&v).is_empty(), "{v}");
    assert_eq!(v["scope"]["dependents"], serde_json::json!([]), "{v}");
}

#[test]
fn a_pre_existing_disagreement_is_not_pulled_in_by_a_change_that_does_not_move_a_child() {
    let dir = repo(
        "base-old",
        &on(),
        &[(1, "Planned", ""), (2, "Active", "WORK-1")],
    );
    std::fs::write(
        dir.join(path(2)),
        format!("{}\nMore prose.\n", record(2, "Active", "WORK-1")),
    )
    .unwrap();
    let (_, _, v) = run(&dir, &["check", "--base", "HEAD"]);
    assert!(findings(&v).is_empty(), "{v}");
}

#[test]
fn a_configuration_that_cannot_support_the_rule_says_so_once() {
    let none = "schema_version: 2
rules:
  status.children-agreement: error
record_types:
  work:
    dir: docs/work
    prefix: WORK
    header_shape: yaml-frontmatter
    required_fields: [Status]
    statuses:
      values: [Open, Done]
";
    let dir = repo("no-part-of", none, &[(1, "Open", ""), (2, "Done", "")]);
    let (_, _, v) = run(&dir, &["check"]);
    assert_eq!(notices(&v).len(), 1, "{v}");
    assert!(notices(&v)[0]["message"]
        .as_str()
        .unwrap()
        .contains("no record type declares a `part_of: true`"));
    assert!(findings(&v).is_empty());

    let no_progress = config(
        "rules:\n  status.children-agreement: error\n",
        "    statuses:\n      values: [Planned, Done]\n      success: [Done]\n",
        "",
    );
    let dir = repo(
        "no-progress",
        &no_progress,
        &[
            (1, "Planned", ""),
            (2, "Done", "WORK-1"),
            (3, "Done", "WORK-1"),
        ],
    );
    let (_, _, v) = run(&dir, &["check"]);
    assert_eq!(notices(&v).len(), 1, "{v}");
    assert!(notices(&v)[0]["message"]
        .as_str()
        .unwrap()
        .contains("declare no `statuses.progress`"));
    assert!(findings(&v).is_empty(), "{v}");
}

#[test]
fn a_repository_that_has_not_opted_in_sees_no_finding_and_no_notice() {
    let other = "rules:\n  header.required-fields: error\n";
    let dir = repo(
        "off",
        &config(other, STATUSES, ""),
        &[(1, "Planned", ""), (2, "Active", "WORK-1")],
    );
    let (code, _, v) = run(&dir, &["check"]);
    assert_eq!(code, Some(0), "{v}");
    assert!(findings(&v).is_empty() && notices(&v).is_empty(), "{v}");
    assert!(v.get("notices").is_none(), "{v}");
    let row = v["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == RULE)
        .unwrap();
    assert_eq!(row["status"], "not-enabled", "{v}");
    let declared_off = config(
        "rules:\n  header.required-fields: error\n  status.children-agreement: off\n",
        STATUSES,
        "",
    );
    set_config(&dir, &declared_off);
    let (_, _, v) = run(&dir, &["check"]);
    assert!(findings(&v).is_empty() && notices(&v).is_empty(), "{v}");
}

fn set_config(dir: &Path, config: &str) {
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
}

#[test]
fn the_export_flag_and_the_rule_agree_on_which_records_disagree() {
    let dir = repo(
        "export",
        &on(),
        &[
            (1, "Planned", ""),
            (2, "Active", "WORK-1"),
            (3, "Done", ""),
            (4, "Planned", "WORK-3"),
            (5, "Active", ""),
            (6, "Done", "WORK-5"),
        ],
    );
    let (_, _, check) = run(&dir, &["check"]);
    let (_, _, export) = run(&dir, &["export", "--format", "json"]);
    let mut from_rule: Vec<String> = findings(&check)
        .iter()
        .map(|f| f["file"].as_str().unwrap().to_string())
        .collect();
    let mut from_export: Vec<String> = export["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["state_disagrees_with_children"] == true)
        .map(|r| r["path"].as_str().unwrap().to_string())
        .collect();
    from_rule.sort();
    from_export.sort();
    assert_eq!(from_rule, vec![path(1), path(3)]);
    assert_eq!(from_rule, from_export);
}

#[test]
fn a_parent_whose_status_is_not_declared_is_not_taken_for_one_not_started() {
    let dir = repo(
        "undeclared",
        &on(),
        &[(1, "Someday", ""), (2, "Active", "WORK-1")],
    );
    let (_, _, check) = run(&dir, &["check"]);
    assert!(findings(&check).is_empty(), "{check}");
    let (_, _, export) = run(&dir, &["export", "--format", "json"]);
    let parent = export["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["path"] == path(1))
        .unwrap();
    assert_eq!(parent["state_disagrees_with_children"], false, "{parent}");
    assert_eq!(
        parent["state"], "not-started",
        "the export's state is unchanged"
    );
}

// ---------------------------------------------------------- the edit-time report

const HARNESS: &str = "harness:
  adapters: [claude-code]
  instructions_file: AGENTS.md
  guardrails:
    editor-hook: {enabled: true}
    post-command: {enabled: true}
";

fn post(dir: &Path) -> Value {
    let request = serde_json::json!({
        "event": "post-command", "tool": "Bash", "paths": [], "actor": "agent",
        "cwd": dir.to_string_lossy(),
    })
    .to_string();
    let mut child = common::urzua()
        .args(["harness", "run", "post-command"])
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
        .write_all(request.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "never blocks: {out:?}");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {out:?}"))
}

fn harness_repo(name: &str, rule: &str, records: &[(u32, &str, &str)]) -> common::TempDir {
    let dir = repo(name, &config(rule, STATUSES, HARNESS), records);
    std::fs::write(dir.join("AGENTS.md"), "# Agents\n").unwrap();
    common::commit_all(&dir, 0);
    dir
}

#[test]
fn a_command_that_moves_a_child_is_told_the_parent_is_still_not_started() {
    let dir = harness_repo(
        "post",
        "rules:\n  status.children-agreement: error\n",
        &[
            (1, "Planned", ""),
            (2, "Planned", "WORK-1"),
            (3, "Planned", "WORK-1"),
            (4, "Done", ""),
            (5, "Done", "WORK-4"),
        ],
    );
    set(&dir, 2, "Active", "WORK-1");
    set(&dir, 3, "Done", "WORK-1");
    set(&dir, 5, "Active", "WORK-4");
    let v = post(&dir);
    assert_eq!(v["verdict"], "allow-with-warnings", "{v}");
    let lines = v["parent_state"].as_array().unwrap();
    assert_eq!(lines.len(), 2, "one line per parent: {v}");
    let text = lines
        .iter()
        .map(|l| l.as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        text.contains(
            "WORK-1 is still 'Planned' while WORK-2 is now 'Active' (and 1 more of its children)"
        ),
        "{text}"
    );
    assert!(
        text.contains("WORK-4 is 'Done', which finishes it, while WORK-5 is now 'Active'"),
        "{text}"
    );
    assert!(
        v["agent"]
            .as_str()
            .unwrap()
            .contains("WORK-1 is still 'Planned'"),
        "{v}"
    );
    assert!(
        v["message"]
            .as_str()
            .unwrap()
            .contains("WORK-1 is still 'Planned'"),
        "{v}"
    );
}

#[test]
fn the_report_is_silent_when_the_rule_is_off_or_the_parent_agrees() {
    let off = harness_repo(
        "post-off",
        "rules:\n  header.required-fields: error\n",
        &[(1, "Planned", ""), (2, "Planned", "WORK-1")],
    );
    set(&off, 2, "Active", "WORK-1");
    let v = post(&off);
    assert!(v.get("parent_state").is_none(), "{v}");
    assert_eq!(v["verdict"], "allow", "{v}");

    let agrees = harness_repo(
        "post-agree",
        "rules:\n  header.required-fields: error\n  status.children-agreement: error\n",
        &[(1, "Active", ""), (2, "Planned", "WORK-1")],
    );
    set(&agrees, 2, "Active", "WORK-1");
    let v = post(&agrees);
    assert!(v.get("parent_state").is_none(), "{v}");
    assert_eq!(v["verdict"], "allow", "{v}");
}

#[test]
fn a_parent_the_command_changed_too_is_told_once() {
    let dir = harness_repo(
        "post-both",
        "rules:\n  status.children-agreement: error\n",
        &[(1, "Planned", ""), (2, "Planned", "WORK-1")],
    );
    set(&dir, 2, "Active", "WORK-1");
    std::fs::write(
        dir.join(path(1)),
        format!("{}\nMore prose.\n", record(1, "Planned", "")),
    )
    .unwrap();
    let v = post(&dir);
    let new_findings = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == RULE)
        .count();
    let lines = v["parent_state"].as_array().map_or(0, Vec::len);
    assert_eq!(new_findings + lines, 1, "{v}");
}

fn edit(dir: &Path, n: u32, content: &str) -> (i32, Value) {
    let request = serde_json::json!({
        "event": "edit", "tool": "Write", "paths": [path(n)], "content": content,
        "actor": "agent", "cwd": dir.to_string_lossy(),
    })
    .to_string();
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
        .write_all(request.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let v = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {out:?}"));
    (out.status.code().unwrap(), v)
}

#[test]
fn an_edit_that_moves_a_child_is_allowed_and_names_the_parent_it_contradicts() {
    let dir = harness_repo(
        "edit",
        "rules:\n  header.required-fields: error\n  status.children-agreement: error\n",
        &[(1, "Planned", ""), (2, "Planned", "WORK-1")],
    );
    let (code, v) = edit(&dir, 2, &record(2, "Active", "WORK-1"));
    assert_eq!(
        code, 0,
        "an edit is never blocked for a parent's state: {v}"
    );
    assert_eq!(v["verdict"], "allow-with-warnings", "{v}");
    let m = v["message"].as_str().unwrap();
    assert!(
        m.contains("WORK-1 is still 'Planned' while WORK-2 is now 'Active'"),
        "{m}"
    );

    let (code, v) = edit(&dir, 2, &record(2, "Planned", "WORK-1"));
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["verdict"], "allow", "{v}");
    assert!(
        !v["message"].as_str().unwrap().contains("WORK-1 is still"),
        "{v}"
    );
}

#[test]
fn an_edit_says_nothing_about_the_parent_when_the_rule_is_off() {
    let dir = harness_repo(
        "edit-off",
        "rules:\n  header.required-fields: error\n",
        &[(1, "Planned", ""), (2, "Planned", "WORK-1")],
    );
    let (_, v) = edit(&dir, 2, &record(2, "Active", "WORK-1"));
    assert_eq!(v["verdict"], "allow", "{v}");
    assert!(
        !v["message"].as_str().unwrap().contains("WORK-1 is still"),
        "{v}"
    );
}

#[test]
fn a_blocked_edit_still_names_the_parent_it_contradicts() {
    let dir = harness_repo(
        "edit-blocked",
        "rules:\n  pointer.resolution: error\n  status.children-agreement: error\n",
        &[(1, "Planned", ""), (2, "Planned", "WORK-1")],
    );
    let (code, v) = edit(&dir, 2, &record(2, "Active", "WORK-1, WORK-99"));
    assert_ne!(code, 0, "the dangling pointer blocks: {v}");
    assert_eq!(v["verdict"], "block", "{v}");
    assert!(
        v["message"]
            .as_str()
            .unwrap()
            .contains("WORK-1 is still 'Planned' while WORK-2 is now 'Active'"),
        "{v}"
    );
}

#[test]
fn a_prefix_lines_childs_base_status_is_read_so_a_body_change_does_not_pull_in_the_parent() {
    let cfg = on().replace(
        "    header_shape: yaml-frontmatter\n",
        "    fields: {from: prefix-lines}\n",
    );
    let doc = |n: u32, status: &str, part_of: &str| {
        let line = if part_of.is_empty() {
            String::new()
        } else {
            format!("Part-of: {part_of}\n")
        };
        format!("# {n} - Work\n\nStatus: {status}\n{line}\nprose\n")
    };
    let dir = repo("prefix-lines", &cfg, &[]);
    std::fs::write(dir.join(path(1)), doc(1, "Planned", "")).unwrap();
    std::fs::write(dir.join(path(2)), doc(2, "Active", "WORK-1")).unwrap();
    common::commit_all(&dir, 1);
    let (_, _, full) = run(&dir, &["check"]);
    assert_eq!(findings(&full).len(), 1, "precondition: {full}");
    std::fs::write(
        dir.join(path(2)),
        format!("{}\nMore prose.\n", doc(2, "Active", "WORK-1")),
    )
    .unwrap();
    let (_, _, v) = run(&dir, &["check", "--base", "HEAD"]);
    assert!(findings(&v).is_empty(), "{v}");
    assert_eq!(v["scope"]["dependents"], serde_json::json!([]), "{v}");
}
