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

fn to_fill_entries(report: &serde_json::Value) -> Vec<serde_json::Value> {
    report["to_fill"].as_array().cloned().unwrap_or_default()
}

/// Every finding about a header field `new` left empty is `field.quality`'s
/// true "Blank" on a required field; an optional one draws nothing.
fn only_blank_required(findings: &[serde_json::Value], report: &serde_json::Value) {
    let entries = to_fill_entries(report);
    let named = |field: &str| entries.iter().find(|e| e["field"] == field).cloned();
    for f in findings {
        let message = f["message"].as_str().unwrap_or_default();
        for e in &entries {
            let field = e["field"].as_str().unwrap();
            let about = message.contains(&format!("'{field}'"))
                || message.starts_with(&format!("{field} "));
            if !about {
                continue;
            }
            assert_eq!(f["rule"], "field.quality", "{f}");
            assert!(message.contains("is Blank"), "{f}");
            assert_eq!(named(field).unwrap()["required"], true, "{f}");
        }
        assert_ne!(f["rule"], "header.pointer-field-clean", "{f}");
    }
}

fn assert_header_invents_nothing(text: &str, report: &serde_json::Value) {
    let head = header(text);
    assert!(!head.contains("null"), "{text}");
    for e in to_fill_entries(report) {
        let field = e["field"].as_str().unwrap();
        let line = head
            .lines()
            .find(|l| {
                l.trim_matches(|c| c == '\'' || c == '"')
                    .starts_with(&format!("{field}:"))
            })
            .unwrap_or_else(|| panic!("{field} not written: {text}"));
        assert!(line.trim_end().ends_with(':'), "{field} not empty: {line}");
    }
}

#[test]
fn every_initialised_profile_writes_empty_fields_and_check_reports_only_blank_required_ones() {
    let dir = repo("profiles");
    run(dir.path(), &["init", "--types", "adr,rfc,spec,prd"]);
    common::git(&dir, &["add", "-A"]);
    common::git(&dir, &["commit", "-q", "-m", "init"]);
    for t in ["adr", "rfc", "spec", "prd"] {
        let (report, text) = new_staged(dir.path(), t);
        assert_header_invents_nothing(&text, &report);
        let path = report["path"].as_str().unwrap();
        only_blank_required(&findings_on(dir.path(), path), &report);
    }
}

#[test]
fn the_adr_profile_leaves_deciders_empty_and_hands_its_hint_to_the_author() {
    let dir = repo("adr-hint");
    run(dir.path(), &["init", "--types", "adr"]);
    common::git(&dir, &["add", "-A"]);
    common::git(&dir, &["commit", "-q", "-m", "init"]);
    let report = run(dir.path(), &["new", "adr", "First thing", "--by", "tester"]);
    let text = std::fs::read_to_string(dir.join(report["path"].as_str().unwrap())).unwrap();
    assert!(text.contains("\nDeciders:\n"), "{text}");
    assert!(!text.contains("name(s)"), "{text}");
    let entry = to_fill_entries(&report)
        .into_iter()
        .find(|e| e["field"] == "Deciders")
        .unwrap_or_else(|| panic!("{report}"));
    assert_eq!(entry["hint"], "name(s)", "{report}");
    assert!(entry["required"].is_boolean(), "{report}");
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
    known_fields: [Parent, Up, Down, Decided, Labels, Notes]
    pointer_fields: [{name: Parent, targets: [task]}]
    narrative_fields: [Notes]
    field_values:
      Labels: [ui, api]
    statuses:
      values: [Todo, InProgress, Done]
      initial: [Todo]
      transitions:
        Todo: [InProgress]
        InProgress: [Done]
relation_pairs:
  - {name: link, forward: Up, inverse: Down, reciprocity: required, cross_type: false, types: [task], target_types: [task]}
rules:
  header.required-fields: error
  header.pointer-field-clean: warn
  header.field-set-consistency: error
  pointer.resolution: error
  pointer.target-type: error
  pointer.target-status:
    level: warn
    not_in: [Done]
  relation.pair-reciprocity: error
  relation.cross-type-disallowed: error
  field.quality: warn
  field.pending: warn
  field.value-not-declared: error
  field.untrimmed-value: warn
  field.leading-reserved-indicator: warn
  narrative-field.stale: warn
  date.field-valid:
    level: error
    date_fields: [Decided]
  status.undeclared-value: error
";

const CUSTOM_TEMPLATE: &str = "---
Status: Todo | InProgress | Done
Kind: feature | bug | chore
Parent: the task this one belongs to
Up: the task this one rolls up into
Down:
Decided: YYYY-MM-DD
Labels: ui | api
Notes: anything a reader needs
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

/// Pointer, relation, date, vocabulary and narrative fields left empty and
/// optional draw no finding from any of their rules; the empty required fields
/// draw `field.quality`'s true "Blank".
#[test]
fn empty_optional_fields_of_every_kind_trip_no_rule_and_empty_required_ones_are_blank() {
    let dir = custom_repo("custom");
    let (report, text) = new_staged(dir.path(), "task");
    assert_header_invents_nothing(&text, &report);
    assert!(!text.contains("feature"), "{text}");
    let fields: Vec<String> = to_fill_entries(&report)
        .iter()
        .map(|e| e["field"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        fields,
        ["Kind", "Owner", "Parent", "Up", "Down", "Decided", "Labels", "Notes"],
        "{report}"
    );
    let path = report["path"].as_str().unwrap();
    let findings = findings_on(dir.path(), path);
    only_blank_required(&findings, &report);
    let blank: Vec<&str> = findings
        .iter()
        .filter(|f| f["rule"] == "field.quality")
        .map(|f| f["message"].as_str().unwrap())
        .collect();
    assert_eq!(blank.len(), 2, "{findings:?}");
}

/// An empty optional pointer written by hand, as `Field:` or as an existing
/// record's `Field: null`, is no reference, not an unclean one.
#[test]
fn a_hand_written_empty_or_null_optional_pointer_is_not_reported() {
    let dir = custom_repo("hand");
    for (n, empty) in [(1, "Parent:"), (2, "Parent: null")] {
        write(
            dir.path(),
            &format!("docs/tasks/TASK-{n}-hand.md"),
            &format!("---\nStable-Id: 01HAND{n}\nStatus: Todo\nKind: bug\nOwner: me\n{empty}\n---\n# TASK-{n} — Hand\n\n## What\n\nx\n"),
        );
    }
    common::git(&dir, &["add", "-A"]);
    let report = run(dir.path(), &["check"]);
    let about_parent: Vec<_> = report["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("{report}"))
        .iter()
        .filter(|f| f["message"].as_str().unwrap_or_default().contains("Parent"))
        .collect();
    assert!(about_parent.is_empty(), "{about_parent:?}");
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
    std::fs::write(
        dir.join(path),
        text.replace("\nOwner:\n", "\nOwner: Todo\n"),
    )
    .unwrap();
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

fn doctor_checks(config: &str) -> (serde_json::Value, Vec<serde_json::Value>) {
    let dir = repo("doctor-vocab");
    write(dir.path(), ".urzua/config.yaml", config);
    std::fs::create_dir_all(dir.join("docs/t")).unwrap();
    let report = run(dir.path(), &["doctor"]);
    let checks = report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["check"] == "declared-value-vocabulary")
        .cloned()
        .collect();
    (report, checks)
}

/// A type that lists its own declared status under `statuses.placeholders`
/// and `statuses.pending` contradicts itself: `warn`, naming both.
#[test]
fn doctor_warns_when_a_type_lists_its_own_status_as_a_placeholder_or_pending() {
    let (report, checks) = doctor_checks(
        "schema_version: 2\nrecord_types:\n  t:\n    dir: docs/t\n    required_fields: [Status]\n    statuses:\n      values: [Draft, Done]\n      placeholders: [Draft]\n      pending: [Draft]\n",
    );
    assert_eq!(checks.len(), 1, "{report}");
    assert_eq!(checks[0]["status"], "warn", "{report}");
    let message = checks[0]["message"].as_str().unwrap();
    assert!(
        message.contains("'Draft'")
            && message.contains("placeholder")
            && message.contains("pending"),
        "{message}"
    );
}

/// A value a type declares under `field_values` is walked too.
#[test]
fn doctor_names_a_field_values_entry_in_a_default_vocabulary() {
    let (report, checks) = doctor_checks(
        "schema_version: 2\nrecord_types:\n  t:\n    dir: docs/t\n    required_fields: [Kind]\n    known_fields: [Kind]\n    field_values:\n      Kind: [feature, tbd]\n",
    );
    assert_eq!(checks.len(), 1, "{report}");
    assert_eq!(checks[0]["status"], "info", "{report}");
    let message = checks[0]["message"].as_str().unwrap();
    assert!(
        message.contains("'Kind'") && message.contains("'tbd'"),
        "{message}"
    );
}
