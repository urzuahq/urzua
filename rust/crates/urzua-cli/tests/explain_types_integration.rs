//! `urzua explain --types` and `urzua explain --type <name>`:
//! what the configuration declares for its record types, each entry with the
//! key path and line it was read from.

mod common;

use std::path::Path;
use std::process::Command;

use serde_json::Value;

const FIXTURE: &str = include_str!("fixtures/explain_types/config.yaml");

/// The fixture's report, pretty-printed as the command prints it, stays under
/// this many bytes.
const MAX_REPORT_BYTES: usize = 12_000;

fn repo(name: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-explain-types-{name}"));
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    for d in [
        ".urzua/templates",
        "docs/issues",
        "docs/tasks",
        "docs/notes",
    ] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    dir
}

fn run(dir: &Path, args: &[&str]) -> (std::process::Output, Value) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("not JSON ({e}): {}", String::from_utf8_lossy(&out.stdout)));
    (out, v)
}

fn of<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == name)
        .unwrap_or_else(|| panic!("no type {name}: {report}"))
}

fn field<'a>(t: &'a Value, name: &str) -> &'a Value {
    t["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == name)
        .unwrap_or_else(|| panic!("no field {name}: {t}"))
}

/// Every object holding `at` names a key path and a 1-based line; no marked
/// default carries one.
fn assert_positions(v: &Value) {
    match v {
        Value::Object(m) => {
            if m.get("default") == Some(&Value::Bool(true)) {
                assert!(!m.contains_key("at"), "a default carries no position: {v}");
            }
            if let Some(at) = m.get("at") {
                let at = at.as_str().unwrap();
                let (_, line) = at
                    .rsplit_once(':')
                    .unwrap_or_else(|| panic!("no line: {at}"));
                assert!(line.parse::<usize>().unwrap() > 0, "{at}");
            }
            m.values().for_each(assert_positions);
        }
        Value::Array(a) => a.iter().for_each(assert_positions),
        _ => {}
    }
}

#[test]
fn every_declaration_is_reported_with_its_position() {
    let dir = repo("full", FIXTURE);
    std::fs::write(dir.join(".urzua/templates/issue.md"), "# {title}\n").unwrap();
    let (out, r) = run(&dir, &["explain", "--types"]);
    assert_eq!(out.status.code(), Some(0), "{r}");
    assert_eq!(r["status"], "ok");
    assert_positions(&r);
    let issue = of(&r, "issue");
    assert_eq!(issue["create"], "urzua new issue \"<title>\"");
    assert_eq!(issue["prefix"]["value"], "ISS");
    assert_eq!(issue["prefix"]["at"], "record_types.issue.prefix:6");
    assert_eq!(issue["dirs"][1]["value"], "docs/issues-archive");
    assert_eq!(issue["header_shape"]["value"], "yaml-frontmatter");
    assert_eq!(issue["template"]["path"], ".urzua/templates/issue.md");
    assert_eq!(field(issue, "Kind")["required"], true);
    assert_eq!(
        field(issue, "Kind")["values"]["value"],
        serde_json::json!(["bug", "chore"])
    );
    assert!(field(issue, "Severity").get("required").is_none());
    assert_eq!(
        field(issue, "Fixes")["pointer"]["targets"],
        serde_json::json!(["task"])
    );
    assert!(field(issue, "See-also")["narrative"]["at"].is_string());
    assert_eq!(field(issue, "State")["roles"][0]["role"], "status");
    assert_eq!(field(issue, "Opened")["roles"][0]["role"], "date");
    let states = &issue["states"];
    assert_eq!(states["initial"]["value"], serde_json::json!(["Open"]));
    assert_eq!(states["transitions"][0]["from"], "Closed");
    assert_eq!(states["failure"]["value"], serde_json::json!(["Dropped"]));
    let require = &issue["conditions"][0];
    assert_eq!(require["kind"], "require");
    assert_eq!(require["when"], serde_json::json!(["Closed"]));
    assert_eq!(require["when_field"]["Kind"], serde_json::json!(["bug"]));
    assert_eq!(
        require["sentence"],
        "when State is Closed and Kind is bug, Severity is required"
    );
    assert_eq!(
        issue["conditions"][1]["sentence"],
        "when State is Open, Fixes is forbidden"
    );
    assert_eq!(issue["gates"][0]["to"], serde_json::json!(["Dropped"]));
    assert_eq!(issue["gates"][0]["logins"], serde_json::json!(["someone"]));
    assert!(issue["revision_log"]["at"].is_string());
    assert_eq!(
        of(&r, "note")["gates"][0]["paths"],
        serde_json::json!(["docs/notes"])
    );
    let task = of(&r, "task");
    assert_eq!(
        task["relation_pairs"][0]["target_types"],
        serde_json::json!(["task"])
    );
    assert_eq!(
        task["states"]["initial"],
        serde_json::json!({"value": ["Todo"], "default": true})
    );
}

#[test]
fn rules_set_on_by_declaration_and_declared_but_not_enabled() {
    let dir = repo("rules", FIXTURE);
    let (_, r) = run(&dir, &["explain", "--types"]);
    let rules = &r["rules"];
    let set = rules["set"].as_array().unwrap();
    assert_eq!(set.len(), 2);
    assert_eq!(set[1]["id"], "status.transition-allowed");
    assert_eq!(set[1]["level"], "warn");
    let on = rules["on_by_declaration"].as_array().unwrap();
    let values = on
        .iter()
        .find(|e| e["id"] == "field.value-not-declared")
        .unwrap();
    assert_eq!(values["default"], true);
    assert_eq!(
        values["declared_by"][0],
        "record_types.issue.field_values:11"
    );
    let off = rules["declared_but_not_enabled"].as_array().unwrap();
    let missing = off
        .iter()
        .find(|e| e["id"] == "status.required-field-missing")
        .unwrap();
    assert_eq!(
        missing["declared_by"][0],
        "record_types.issue.statuses.require:25"
    );
    assert!(off.iter().all(|e| e["id"] != "status.transition-allowed"));
}

#[test]
fn one_type_and_an_unknown_type() {
    let dir = repo("one", FIXTURE);
    let (out, r) = run(&dir, &["explain", "--type", "task"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(r["types"].as_array().unwrap().len(), 1);
    let (out, r) = run(&dir, &["explain", "--type", "nope"]);
    assert_eq!(out.status.code(), Some(2), "{r}");
    assert_eq!(r["status"], "not-run");
    let message = r["error"].as_str().unwrap();
    assert!(message.contains("issue, note, task"), "{message}");
}

#[test]
fn the_report_stays_under_its_size() {
    let dir = repo("size", FIXTURE);
    let out = common::urzua()
        .args(["explain", "--types"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        out.stdout.len() < MAX_REPORT_BYTES,
        "{} bytes for the three-type fixture",
        out.stdout.len()
    );
}

#[test]
fn a_bare_type_prints_nothing_it_does_not_declare() {
    let dir = repo(
        "bare",
        "schema_version: 2\nrecord_types:\n  note:\n    dir: docs/notes\n",
    );
    let (_, r) = run(&dir, &["explain", "--types"]);
    let note = of(&r, "note");
    let mut keys: Vec<&String> = note.as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(keys, ["at", "create", "dirs", "name"], "{note}");
    assert_eq!(r["rules"], serde_json::json!({}), "{r}");
}

#[test]
fn this_repository_configuration_loads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let (out, r) = run(&root, &["explain", "--types"]);
    assert_eq!(out.status.code(), Some(0), "{r}");
    for name in ["adr", "spec", "milestone", "bug"] {
        assert!(of(&r, name)["dirs"][0]["at"].is_string());
    }
    assert_positions(&r);
}
