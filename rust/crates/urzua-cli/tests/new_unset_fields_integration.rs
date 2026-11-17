//! `MILE-322` and `MILE-324`: a record `urzua new` writes starts with no finding
//! the author did not cause. A field `new` has no value for carries the
//! template's text, the `—` sentinel or nothing, never `null`; a status value
//! the type declares is a real value in the status field.

mod common;

use std::path::Path;

fn repo(label: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-cli-new-unset-{label}"));
    common::git(&dir, &["init", "-q"]);
    common::git(&dir, &["config", "user.email", "test@example.com"]);
    common::git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join("README.md"), "# project\n").unwrap();
    common::git(&dir, &["add", "-A"]);
    common::git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn write(dir: &Path, rel: &str, body: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

fn run(dir: &Path, args: &[&str]) -> serde_json::Value {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("not JSON: {e}\n{stdout}"))
}

/// `new`, then stage it as its report says: the report and the record's text.
fn new_staged(dir: &Path, record_type: &str) -> (serde_json::Value, String) {
    let report = run(dir, &["new", record_type, "First thing", "--by", "tester"]);
    let path = report["path"]
        .as_str()
        .unwrap_or_else(|| panic!("{report}"));
    common::git(dir, &["add", "--", path]);
    (
        report.clone(),
        std::fs::read_to_string(dir.join(path)).unwrap(),
    )
}

fn header(text: &str) -> &str {
    let body = text.strip_prefix("---\n").unwrap_or(text);
    &body[..body.find("\n---\n").unwrap_or(body.len())]
}

/// The findings `check` reports on `path`.
fn findings_on(dir: &Path, path: &str) -> Vec<serde_json::Value> {
    let report = run(dir, &["check"]);
    report["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("{report}"))
        .iter()
        .filter(|f| f["file"].as_str() == Some(path) || f["path"].as_str() == Some(path))
        .cloned()
        .collect()
}

fn to_fill(report: &serde_json::Value) -> Vec<String> {
    report["to_fill"]
        .as_array()
        .unwrap_or_else(|| panic!("{report}"))
        .iter()
        .map(|f| f["field"].as_str().unwrap().to_string())
        .collect()
}

/// Every finding about a header field is a placeholder `new` listed to fill.
fn only_listed_placeholders(findings: &[serde_json::Value], listed: &[String]) {
    for f in findings {
        let rule = f["rule"].as_str().unwrap_or_default();
        let message = f["message"].as_str().unwrap_or_default();
        assert_ne!(rule, "header.pointer-field-clean", "{f}");
        assert!(!message.contains("is Blank"), "{f}");
        if rule == "field.quality" {
            let field = message.split('\'').nth(1).unwrap_or_default();
            assert!(message.contains("is Placeholder"), "{f}");
            assert!(listed.iter().any(|l| l == field), "{f} not in {listed:?}");
        }
    }
}

#[test]
fn every_initialised_profile_writes_no_null_and_check_reports_only_listed_placeholders() {
    let dir = repo("profiles");
    run(dir.path(), &["init", "--types", "adr,rfc,spec,prd"]);
    common::git(&dir, &["add", "-A"]);
    common::git(&dir, &["commit", "-q", "-m", "init"]);
    for t in ["adr", "rfc", "spec", "prd"] {
        let (report, text) = new_staged(dir.path(), t);
        assert!(!header(&text).contains("null"), "{t}: {text}");
        let path = report["path"].as_str().unwrap();
        only_listed_placeholders(&findings_on(dir.path(), path), &to_fill(&report));
    }
}

#[test]
fn the_adr_profile_carries_the_deciders_hint_and_lists_it_with_a_next_action() {
    let dir = repo("adr-hint");
    run(dir.path(), &["init", "--types", "adr"]);
    common::git(&dir, &["add", "-A"]);
    common::git(&dir, &["commit", "-q", "-m", "init"]);
    let report = run(dir.path(), &["new", "adr", "First thing", "--by", "tester"]);
    let text = std::fs::read_to_string(dir.join(report["path"].as_str().unwrap())).unwrap();
    assert!(text.contains("\nDeciders: name(s)\n"), "{text}");
    assert!(
        to_fill(&report).contains(&"Deciders".to_string()),
        "{report}"
    );
    let reason = report["next_action_reason"].as_str().unwrap_or_default();
    assert!(reason.contains("Deciders"), "{report}");
    assert!(
        report["next_action"]
            .as_str()
            .unwrap()
            .starts_with("git add"),
        "{report}"
    );
}

const CUSTOM_CONFIG: &str = "schema_version: 2
record_types:
  task:
    dir: docs/tasks
    prefix: TASK
    header_shape: yaml-frontmatter
    required_fields: [Status, Kind, Owner]
    known_fields: [Parent]
    pointer_fields: [Parent]
    statuses:
      values: [Todo, InProgress, Done]
      initial: [Todo]
      transitions:
        Todo: [InProgress]
        InProgress: [Done]
rules:
  header.required-fields: error
  header.pointer-field-clean: warn
  field.quality: warn
  field.pending: warn
";

const CUSTOM_TEMPLATE: &str = "---
Status: Todo | InProgress | Done
Kind: feature | bug | chore
Parent: the task this one belongs to
---
# NNNN — Title

## What

x
";

fn custom_repo(label: &str) -> common::TempDir {
    let dir = repo(label);
    write(dir.path(), ".urzua/config.yaml", CUSTOM_CONFIG);
    write(dir.path(), ".urzua/templates/task.md", CUSTOM_TEMPLATE);
    std::fs::create_dir_all(dir.join("docs/tasks")).unwrap();
    common::git(&dir, &["add", "-A"]);
    common::git(&dir, &["commit", "-q", "-m", "config"]);
    dir
}

#[test]
fn a_required_field_keeps_its_template_text_and_an_unset_optional_pointer_trips_nothing() {
    let dir = custom_repo("custom");
    let (report, text) = new_staged(dir.path(), "task");
    let head = header(&text);
    assert!(!head.contains("null"), "{text}");
    assert!(head.contains("Kind: feature | bug | chore"), "{text}");
    assert!(head.contains("Owner: —"), "{text}");
    assert!(!head.contains("Parent"), "{text}");
    let listed = to_fill(&report);
    assert_eq!(listed, ["Kind", "Owner"], "{report}");
    let findings = findings_on(dir.path(), report["path"].as_str().unwrap());
    only_listed_placeholders(&findings, &listed);
}

/// The declared status `Todo` is in both default vocabularies, yet `new`,
/// stage and `check` report nothing about `Status`.
#[test]
fn a_declared_status_in_the_default_vocabularies_is_clean_after_new() {
    let dir = custom_repo("todo");
    let (report, text) = new_staged(dir.path(), "task");
    assert!(header(&text).contains("Status: Todo"), "{text}");
    let findings = findings_on(dir.path(), report["path"].as_str().unwrap());
    let on_status: Vec<_> = findings
        .iter()
        .filter(|f| {
            f["message"]
                .as_str()
                .unwrap_or_default()
                .contains("'Status'")
        })
        .collect();
    assert!(on_status.is_empty(), "{on_status:?}");
}

/// The same word in a field that declares no values is still reported.
#[test]
fn the_declared_word_in_another_field_is_still_reported() {
    let dir = custom_repo("todo-elsewhere");
    let (report, _) = new_staged(dir.path(), "task");
    let path = report["path"].as_str().unwrap();
    let text = std::fs::read_to_string(dir.join(path)).unwrap();
    std::fs::write(dir.join(path), text.replace("Owner: —", "Owner: Todo")).unwrap();
    common::git(&dir, &["add", "--", path]);
    let findings = findings_on(dir.path(), path);
    assert!(
        findings.iter().any(|f| f["rule"] == "field.quality"
            && f["message"]
                .as_str()
                .unwrap()
                .contains("'Owner' is Placeholder")),
        "{findings:?}"
    );
}

#[test]
fn doctor_names_a_declared_value_in_a_default_vocabulary_at_info() {
    let dir = custom_repo("doctor");
    let report = run(dir.path(), &["doctor"]);
    let info: Vec<&serde_json::Value> = report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["check"] == "declared-value-vocabulary")
        .collect();
    assert_eq!(info.len(), 1, "{report}");
    assert_eq!(info[0]["status"], "info", "{report}");
    let message = info[0]["message"].as_str().unwrap();
    assert!(
        message.contains("'task'") && message.contains("'Todo'"),
        "{message}"
    );
    assert_ne!(report["status"], "info", "{report}");
}
