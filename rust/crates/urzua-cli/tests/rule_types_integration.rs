//! `types:` on a rule entry: the rule judges only records of the listed types,
//! counts the others as out of scope, refuses a name that is no declared type
//! or an empty list at load, says once when a listed type can never be judged,
//! and holds the same way on a plain run, a `--base` run, the editor hook's
//! `--stdin-path`, `fix` and `explain --rule`, composed with `scope: changed`.

mod common;

use serde_json::Value;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;

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

fn config(rules: &str) -> String {
    format!(
        "schema_version: 2
rules:
{rules}record_types:
  work:
    dir: docs/work
    prefix: WORK
    header_shape: yaml-frontmatter
    required_fields: [Status]
    known_fields: [Part-of, Realized-by]
    narrative_fields: []
    pointer_fields:
      - {{name: Part-of, targets: [work], part_of: true}}
{STATUSES}  note:
    dir: docs/note
    prefix: NOTE
    header_shape: yaml-frontmatter
    required_fields: [Status]
    known_fields: [Part-of, Realized-by]
    narrative_fields: []
    pointer_fields:
      - {{name: Part-of, targets: [note], part_of: true}}
{STATUSES}  memo:
    dir: docs/memo
    prefix: MEMO
    header_shape: yaml-frontmatter
    required_fields: [Status]
    known_fields: []
    narrative_fields: []
    pointer_fields: []
{STATUSES}"
    )
}

fn text(prefix: &str, n: u32, header: &str) -> String {
    format!("---\n{header}---\n# {prefix}-{n} - R\n")
}

fn rel(kind: &str, n: u32) -> String {
    let prefix = kind.to_uppercase();
    format!("docs/{kind}/{prefix}-{n}-r.md")
}

fn put(dir: &Path, kind: &str, n: u32, header: &str) {
    let prefix = kind.to_uppercase();
    std::fs::write(dir.join(rel(kind, n)), text(&prefix, n, header)).unwrap();
}

fn repo(name: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-rule-types-{name}"));
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    for kind in ["work", "note", "memo"] {
        std::fs::create_dir_all(dir.join(format!("docs/{kind}"))).unwrap();
    }
    std::fs::write(dir.join(".gitignore"), ".urzua/cache/\n").unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    // Each type: one record without its required Status.
    put(&dir, "work", 1, "Part-of: WORK-2\n");
    put(&dir, "note", 1, "Part-of: NOTE-2\n");
    put(&dir, "work", 2, "Status: Planned\n");
    put(&dir, "note", 2, "Status: Planned\n");
    common::git(&dir, &["init", "-q"]);
    common::commit_all(&dir, 0);
    dir
}

fn run(dir: &Path, args: &[&str]) -> (Option<i32>, Value) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let v = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}: {text}"));
    (out.status.code(), v)
}

fn files_of(v: &Value, rule: &str) -> Vec<String> {
    let mut files: Vec<String> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == rule)
        .map(|f| f["file"].as_str().unwrap().to_string())
        .collect();
    files.sort();
    files
}

fn population<'a>(v: &'a Value, rule: &str) -> &'a Value {
    &v["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == rule)
        .unwrap_or_else(|| panic!("{rule} not executed: {v}"))["population"]
}

const REQUIRED: &str = "header.required-fields";

#[test]
fn a_rule_with_types_judges_only_the_listed_types_and_counts_the_rest_out_of_scope() {
    let dir = repo(
        "judges",
        &config("  header.required-fields:\n    level: error\n    types: [work]\n"),
    );
    let (code, v) = run(&dir, &["check"]);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(files_of(&v, REQUIRED), vec![rel("work", 1)], "{v}");
    let p = population(&v, REQUIRED);
    // One required field, four records: two judged, two outside `types:`.
    assert_eq!(p["eligible"], 4, "{p}");
    assert_eq!(p["examined"], 2, "{p}");
    assert_eq!(p["out_of_scope"], 2, "{p}");
    assert_eq!(p["out_of_types"], 2, "{p}");

    let all = repo("all", &config("  header.required-fields: error\n"));
    let (_, v) = run(&all, &["check"]);
    assert_eq!(
        files_of(&v, REQUIRED),
        vec![rel("note", 1), rel("work", 1)],
        "{v}"
    );
    let p = population(&v, REQUIRED);
    assert_eq!(p["out_of_scope"], 0, "{p}");
    assert!(p.get("out_of_types").is_none(), "{p}");
}

#[test]
fn a_run_that_examined_nothing_because_of_types_says_so() {
    let dir = repo(
        "nothing",
        &config("  header.required-fields:\n    level: error\n    types: [memo]\n"),
    );
    let (code, v) = run(&dir, &["check"]);
    assert_eq!(code, Some(0), "{v}");
    let p = population(&v, REQUIRED);
    assert_eq!(p["examined"], 0, "{p}");
    assert_eq!(p["out_of_types"], 4, "{p}");
    // The scope-matches-nothing rule does not read a deliberate `types:` as a
    // configuration that reaches nothing.
    let with_scope_rule = repo(
        "nothing-scope-rule",
        &config(
            "  header.required-fields:\n    level: error\n    types: [memo]\n  config.scope-matches-nothing: error\n",
        ),
    );
    let (_, v) = run(&with_scope_rule, &["check"]);
    assert!(
        files_of(&v, "config.scope-matches-nothing").is_empty(),
        "{v}"
    );
}

fn load_error(rules: &str) -> String {
    let dir = repo("load", &config(rules));
    let out = common::urzua()
        .arg("check")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn an_unknown_type_an_empty_list_and_a_rule_that_judges_no_record_are_load_errors() {
    let unknown =
        load_error("  header.required-fields:\n    level: error\n    types: [work, wrok]\n");
    assert!(
        unknown.contains("header.required-fields") && unknown.contains("'wrok'"),
        "{unknown}"
    );
    let empty = load_error("  header.required-fields:\n    level: error\n    types: []\n");
    assert!(
        empty.contains("header.required-fields") && empty.contains("types"),
        "{empty}"
    );
    let config_rule =
        load_error("  type.dir-matches-nothing:\n    level: error\n    types: [work]\n");
    assert!(
        config_rule.contains("type.dir-matches-nothing") && config_rule.contains("types"),
        "{config_rule}"
    );
}

#[test]
fn a_listed_type_the_rule_can_never_judge_is_named_in_one_notice() {
    let dir = repo(
        "never",
        &config(
            "  status.children-agreement:\n    level: error\n    types: [work, memo]\n  header.required-fields:\n    level: error\n    types: [memo]\n",
        ),
    );
    let (_, v) = run(&dir, &["check"]);
    let notices: Vec<&Value> = v["notices"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["subject"] == "rule-types-never-judged")
        .collect();
    assert_eq!(notices.len(), 1, "{v}");
    let m = notices[0]["message"].as_str().unwrap();
    assert!(
        m.contains("status.children-agreement") && m.contains("memo") && !m.contains("work"),
        "{m}"
    );
    assert!(!m.contains("header.required-fields"), "{m}");
}

#[test]
fn a_cross_record_rule_reads_every_record_and_reports_only_on_the_listed_types() {
    let dir = repo(
        "children",
        &config("  status.children-agreement:\n    level: error\n    types: [work]\n"),
    );
    // Both parents are Planned while a child is under way.
    put(&dir, "work", 1, "Status: Active\nPart-of: WORK-2\n");
    put(&dir, "note", 1, "Status: Active\nPart-of: NOTE-2\n");
    let (_, v) = run(&dir, &["check"]);
    assert_eq!(
        files_of(&v, "status.children-agreement"),
        vec![rel("work", 2)],
        "{v}"
    );
}

#[test]
fn a_base_run_and_the_editor_hook_judge_only_the_listed_types() {
    let dir = repo(
        "base",
        &config("  header.required-fields:\n    level: error\n    types: [work]\n"),
    );
    put(&dir, "work", 3, "Part-of: WORK-2\n");
    put(&dir, "note", 3, "Part-of: NOTE-2\n");
    common::git(&dir, &["add", "-A"]);
    let (_, v) = run(&dir, &["check", "--base", "HEAD"]);
    assert_eq!(files_of(&v, REQUIRED), vec![rel("work", 3)], "{v}");

    for (kind, expected) in [("work", 1), ("note", 0)] {
        let path = rel(kind, 4);
        let mut child = common::urzua()
            .args(["check", "--stdin-path", &path])
            .current_dir(&dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let prefix = kind.to_uppercase();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(text(&prefix, 4, "Part-of: X-1\n").as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(files_of(&v, REQUIRED).len(), expected, "{kind}: {v}");
    }
}

#[test]
fn types_composes_with_scope_changed() {
    let rule = "embodiment.locator-promotion-candidate";
    let dir = repo(
        "scope",
        &config(&format!(
            "  {rule}:\n    level: warn\n    scope: changed\n    types: [work]\n"
        )),
    );
    for kind in ["work", "note"] {
        put(
            &dir,
            kind,
            1,
            "Status: Planned\nRealized-by: code:src/shared.rs\n",
        );
        put(
            &dir,
            kind,
            2,
            "Status: Planned\nRealized-by: code:src/shared.rs\n",
        );
    }
    common::commit_all(&dir, 0);
    put(
        &dir,
        "note",
        1,
        "Status: Active\nRealized-by: code:src/shared.rs\n",
    );
    let (_, v) = run(&dir, &["check", "--base", "HEAD"]);
    assert!(files_of(&v, rule).is_empty(), "{v}");
    let p = population(&v, rule);
    assert_eq!(p["out_of_types"], 2, "{p}");
    assert_eq!(p["deferred_by_scope"], 2, "{p}");

    put(
        &dir,
        "work",
        1,
        "Status: Active\nRealized-by: code:src/shared.rs\n",
    );
    let (_, v) = run(&dir, &["check", "--base", "HEAD"]);
    assert_eq!(files_of(&v, rule), vec![rel("work", 1)], "{v}");
}

#[test]
fn explain_rule_shows_the_declared_types() {
    let dir = repo(
        "explain",
        &config("  header.required-fields:\n    level: error\n    types: [work]\n"),
    );
    let (_, v) = run(&dir, &["explain", "--rule", REQUIRED]);
    assert_eq!(v["rule"]["types"], serde_json::json!(["work"]), "{v}");
    let all = repo("explain-all", &config("  header.required-fields: error\n"));
    let (_, v) = run(&all, &["explain", "--rule", REQUIRED]);
    assert!(v["rule"]["types"].is_null(), "{v}");
}
