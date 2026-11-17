//! `MILE-248` (enumerated values) and field-conditioned requirements: a type's declared
//! `field_values` vocabulary, checked by `field.value-not-declared`, and
//! `statuses.require`/`forbid` entries conditioned on another field's value,
//! driven end to end through the built binary.

mod common;

use serde_json::Value;

const CONFIG: &str = "schema_version: 2
rules:
  status.required-field-missing: error
  status.forbidden-field-present: error
record_types:
  work:
    dir: docs/work
    prefix: WORK
    required_fields: [Status, Kind]
    known_fields: [Parent, Severity, Labels]
    pointer_fields: [Parent]
    narrative_fields: []
    field_values:
      Kind: [feature, bug, chore]
      Labels: [ui, api]
    statuses:
      values: [Open, Done]
      require:
        - {when: [Done], fields: [Labels]}
        - {when_field: {Kind: [bug]}, fields: [Severity]}
";

fn repo(name: &str, config: &str, records: &[(&str, &str)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-field-values-{name}"));
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/work")).unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    for (file, header) in records {
        std::fs::write(
            dir.join("docs/work").join(file),
            format!("---\n{header}---\n# 1 - X\n"),
        )
        .unwrap();
    }
    common::git(&dir, &["init", "-q"]);
    common::commit_all(&dir, 0);
    dir
}

fn check(dir: &std::path::Path) -> (Option<i32>, Value) {
    let out = common::urzua()
        .arg("check")
        .current_dir(dir)
        .output()
        .unwrap();
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&out.stdout)));
    (out.status.code(), v)
}

fn findings<'a>(v: &'a Value, rule: &str) -> Vec<&'a Value> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == rule)
        .collect()
}

fn files(v: &Value, rule: &str) -> Vec<String> {
    findings(v, rule)
        .iter()
        .map(|f| f["file"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_value_outside_the_vocabulary_is_an_error_by_default_naming_field_value_and_allowed() {
    let dir = repo(
        "vocab",
        CONFIG,
        &[
            (
                "WORK-1-ok.md",
                "Status: Open\nKind: feature\nLabels: [ui, api]\n",
            ),
            ("WORK-2-typo.md", "Status: Open\nKind: Feature\n"),
            (
                "WORK-3-list.md",
                "Status: Open\nKind: chore\nLabels: [ui, db]\n",
            ),
            ("WORK-4-blank.md", "Status: Open\nKind: \"\"\n"),
        ],
    );
    let (code, v) = check(&dir);
    assert_eq!(code, Some(1), "{v}");
    let rule = "field.value-not-declared";
    let mut hit = files(&v, rule);
    hit.sort();
    assert_eq!(
        hit,
        ["docs/work/WORK-2-typo.md", "docs/work/WORK-3-list.md"],
        "{v}"
    );
    let typo = findings(&v, rule)
        .into_iter()
        .find(|f| f["file"] == "docs/work/WORK-2-typo.md")
        .unwrap();
    let m = typo["message"].as_str().unwrap();
    assert!(
        m.contains("'Kind'") && m.contains("'Feature'") && m.contains("feature, bug, chore"),
        "{m}"
    );
    assert_eq!(typo["severity"], "error");
    let list = findings(&v, rule)
        .into_iter()
        .find(|f| f["file"] == "docs/work/WORK-3-list.md")
        .unwrap();
    assert!(list["message"].as_str().unwrap().contains("'db'"), "{list}");
}

#[test]
fn a_kind_outside_the_vocabulary_is_reported_and_a_bug_requires_severity() {
    let dir = repo(
        "combined",
        CONFIG,
        &[
            (
                "WORK-1-bug-ok.md",
                "Status: Open\nKind: bug\nSeverity: high\n",
            ),
            ("WORK-2-bug-no-severity.md", "Status: Open\nKind: bug\n"),
            ("WORK-3-feature.md", "Status: Open\nKind: feature\n"),
            ("WORK-4-typo.md", "Status: Open\nKind: bgu\n"),
            ("WORK-5-done-no-labels.md", "Status: Done\nKind: chore\n"),
        ],
    );
    let (_, v) = check(&dir);
    assert_eq!(
        files(&v, "field.value-not-declared"),
        ["docs/work/WORK-4-typo.md"],
        "{v}"
    );
    assert_eq!(
        files(&v, "status.required-field-missing"),
        [
            "docs/work/WORK-2-bug-no-severity.md",
            "docs/work/WORK-5-done-no-labels.md"
        ],
        "{v}"
    );
    let missing = findings(&v, "status.required-field-missing");
    let m0 = missing[0]["message"].as_str().unwrap();
    assert!(
        m0.starts_with("Kind is bug, which requires 'Severity'"),
        "{m0}"
    );
    let m1 = missing[1]["message"].as_str().unwrap();
    assert!(
        m1.starts_with("Status is Done, which requires 'Labels'"),
        "{m1}"
    );
}

#[test]
fn a_kind_can_forbid_a_field() {
    let config = CONFIG.replace(
        "        - {when_field: {Kind: [bug]}, fields: [Severity]}\n",
        "        - {when_field: {Kind: [bug]}, fields: [Severity]}\n      forbid:\n        - {when_field: {Kind: [bug]}, fields: [Parent]}\n",
    );
    let dir = repo(
        "forbid",
        &config,
        &[
            (
                "WORK-1-bug.md",
                "Status: Open\nKind: bug\nSeverity: high\nParent: WORK-2\n",
            ),
            (
                "WORK-2-feature.md",
                "Status: Open\nKind: feature\nParent: WORK-1\n",
            ),
        ],
    );
    let (_, v) = check(&dir);
    assert_eq!(
        files(&v, "status.forbidden-field-present"),
        ["docs/work/WORK-1-bug.md"],
        "{v}"
    );
    let m = findings(&v, "status.forbidden-field-present")[0]["message"]
        .as_str()
        .unwrap();
    assert!(m.contains("Kind is bug") && m.contains("'Parent'"), "{m}");
}

#[test]
fn a_declared_value_is_not_read_as_a_placeholder_or_pending_marker() {
    let config = CONFIG
        .replace(
            "rules:\n",
            "rules:\n  field.quality: error\n  field.pending: error\n",
        )
        .replace(
            "Kind: [feature, bug, chore]",
            "Kind: [feature, bug, chore, todo, pending]",
        );
    let dir = repo(
        "markers",
        &config,
        &[
            ("WORK-1-todo.md", "Status: Open\nKind: todo\n"),
            ("WORK-2-pending.md", "Status: Open\nKind: pending\n"),
        ],
    );
    let (_, v) = check(&dir);
    for rule in ["field.quality", "field.pending", "field.value-not-declared"] {
        let on_kind: Vec<_> = findings(&v, rule)
            .into_iter()
            .filter(|f| f["message"].as_str().unwrap().contains("'Kind'"))
            .collect();
        assert!(on_kind.is_empty(), "{rule}: {on_kind:?}");
    }
    let without = CONFIG.replace(
        "rules:\n",
        "rules:\n  field.quality: error\n  field.pending: error\n",
    );
    let dir = repo(
        "markers-off",
        &without.replace("      Kind: [feature, bug, chore]\n", ""),
        &[("WORK-1-todo.md", "Status: Open\nKind: todo\n")],
    );
    let (_, v) = check(&dir);
    assert!(
        !findings(&v, "field.quality").is_empty(),
        "control: without a vocabulary `todo` is a placeholder: {v}"
    );
}

#[test]
fn a_condition_on_an_undeclared_field_or_value_is_a_load_error() {
    for (bad, expect) in [
        (
            "{when_field: {Kind: [bgu]}, fields: [Severity]}",
            "field_values.Kind",
        ),
        (
            "{when_field: {Rank: [high]}, fields: [Severity]}",
            "'Rank', which is in neither",
        ),
    ] {
        let config = CONFIG.replace("{when_field: {Kind: [bug]}, fields: [Severity]}", bad);
        let dir = repo("load", &config, &[]);
        let (code, v) = check(&dir);
        assert_eq!(code, Some(2), "{v}");
        assert!(v.to_string().contains(expect), "{v}");
    }
    let dir = repo(
        "vocab-load",
        &CONFIG.replace("Labels: [ui, api]", "Labels: []"),
        &[],
    );
    let (code, v) = check(&dir);
    assert_eq!(code, Some(2), "{v}");
    assert!(v.to_string().contains("lists no values"), "{v}");
}

fn run(dir: &std::path::Path, args: &[&str]) -> (Option<i32>, Value) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    (
        out.status.code(),
        serde_json::from_slice(&out.stdout).unwrap(),
    )
}

#[test]
fn a_field_is_required_for_one_value_and_forbidden_for_another() {
    let config = CONFIG.replace(
        "        - {when_field: {Kind: [bug]}, fields: [Severity]}\n",
        "        - {when_field: {Kind: [bug]}, fields: [Severity]}\n      forbid:\n        - {when_field: {Kind: [chore]}, fields: [Severity]}\n",
    );
    let dir = repo(
        "req-forbid",
        &config,
        &[
            (
                "WORK-1-bug-ok.md",
                "Status: Open\nKind: bug\nSeverity: high\n",
            ),
            ("WORK-2-bug-missing.md", "Status: Open\nKind: bug\n"),
            ("WORK-3-chore-ok.md", "Status: Open\nKind: chore\n"),
            (
                "WORK-4-chore-carries.md",
                "Status: Open\nKind: chore\nSeverity: low\n",
            ),
        ],
    );
    let (_, v) = check(&dir);
    assert_eq!(
        files(&v, "status.required-field-missing"),
        ["docs/work/WORK-2-bug-missing.md"],
        "{v}"
    );
    assert_eq!(
        files(&v, "status.forbidden-field-present"),
        ["docs/work/WORK-4-chore-carries.md"],
        "{v}"
    );
    let m = findings(&v, "status.forbidden-field-present")[0]["message"]
        .as_str()
        .unwrap();
    assert!(
        m.starts_with("Kind is chore, which forbids 'Severity'"),
        "{m}"
    );
}

#[test]
fn a_status_and_two_field_conditions_are_all_named_in_field_order() {
    let config = CONFIG
        .replace(
            "known_fields: [Parent, Severity, Labels]",
            "known_fields: [Parent, Severity, Labels, Area]",
        )
        .replace(
            "        - {when_field: {Kind: [bug]}, fields: [Severity]}\n",
            "        - {when: [Open], when_field: {Kind: [bug], Area: [ui]}, fields: [Severity]}\n",
        );
    let dir = repo(
        "two-fields",
        &config,
        &[("WORK-1-x.md", "Status: Open\nKind: bug\nArea: ui\n")],
    );
    let (_, v) = check(&dir);
    let m = findings(&v, "status.required-field-missing")[0]["message"]
        .as_str()
        .unwrap();
    assert!(
        m.starts_with("Status is Open and Area is ui and Kind is bug, which requires 'Severity'"),
        "{m}"
    );
}

#[test]
fn selecting_the_rule_where_no_vocabulary_is_declared_runs_the_rest_with_a_notice() {
    let config = CONFIG
        .replace(
            "    field_values:\n      Kind: [feature, bug, chore]\n      Labels: [ui, api]\n",
            "",
        )
        .replacen("rules:\n", "rules:\n  header.required-fields: error\n", 1);
    assert!(!config.contains("field_values"));
    let dir = repo("selection", &config, &[("WORK-1-x.md", "Status: Open\n")]);
    let (code, v) = run(
        &dir,
        &[
            "check",
            "--rules",
            "field.value-not-declared,header.required-fields",
        ],
    );
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(
        files(&v, "header.required-fields"),
        ["docs/work/WORK-1-x.md"],
        "{v}"
    );
    let exec = v["rules_executed"].as_array().unwrap();
    let rule = exec
        .iter()
        .find(|r| r["rule"] == "field.value-not-declared")
        .unwrap();
    assert_eq!(rule["status"], "not-enabled", "{rule}");
    let notice = v["notices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["subject"] == "field.value-not-declared")
        .unwrap_or_else(|| panic!("{v}"));
    assert_eq!(notice["severity"], "info");
    assert!(
        notice["message"]
            .as_str()
            .unwrap()
            .contains("`field_values`"),
        "{notice}"
    );
}

#[test]
fn non_string_scalars_are_read_as_text_and_a_null_value_is_refused_naming_the_key() {
    let config = CONFIG
        .replace("Labels: [ui, api]", "Labels: [1, true]")
        .replace("{when_field: {Kind: [bug]}", "{when_field: {Labels: [1]}");
    let dir = repo(
        "scalars",
        &config,
        &[("WORK-1-x.md", "Status: Open\nKind: bug\nLabels: [1]\n")],
    );
    let (code, v) = check(&dir);
    assert_ne!(code, Some(2), "{v}");
    assert_eq!(
        files(&v, "status.required-field-missing"),
        ["docs/work/WORK-1-x.md"],
        "{v}"
    );
    let null = CONFIG.replace("{when_field: {Kind: [bug]}", "{when_field: {Kind: [~]}");
    let dir = repo("null-when", &null, &[]);
    let (code, v) = check(&dir);
    assert_eq!(code, Some(2), "{v}");
    assert!(
        v["error"]
            .as_str()
            .unwrap()
            .contains("`when_field.Kind` has an entry that is null"),
        "{v}"
    );
}
