//! `urzua explain --types [--full]` and `urzua explain --type <name>`: what the configuration declares for its record types, each
//! entry with the key path and line it was read from.

mod common;

use std::path::Path;
use std::process::Command;

use serde_json::{json, Value};

const FIXTURE: &str = include_str!("fixtures/explain_types/config.yaml");
const EVERY_KEY: &str = include_str!("fixtures/explain_types/every_key.yaml");

/// The summary of the three-type fixture, as printed, stays under this many bytes.
const MAX_SUMMARY_BYTES: usize = 3_500;
/// The full report of the three-type fixture stays under this many bytes.
const MAX_FULL_BYTES: usize = 16_000;
/// The summary of this repository's own configuration stays under this many bytes.
const MAX_OWN_SUMMARY_BYTES: usize = 6_000;

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
    t["header_fields"]
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

fn own_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

#[test]
fn every_declaration_is_reported_with_its_position() {
    let dir = repo("full", FIXTURE);
    std::fs::write(dir.join(".urzua/templates/issue.md"), "# {title}\n").unwrap();
    let (out, r) = run(&dir, &["explain", "--types", "--full"]);
    assert_eq!(out.status.code(), Some(0), "{r}");
    assert_eq!(r["status"], "ok");
    assert_positions(&r);
    let issue = of(&r, "issue");
    assert_eq!(issue["create"], "urzua new issue \"<title>\"");
    assert_eq!(issue["prefix"]["value"], "ISS");
    assert_eq!(issue["prefix"]["at"], "record_types.issue.prefix:6");
    assert_eq!(issue["dirs"][1]["value"], "docs/issues-archive");
    assert_eq!(issue["header_shape"]["value"], "yaml-frontmatter");
    assert_eq!(
        issue["template"],
        json!({"value": ".urzua/templates/issue.md", "default": true, "source": "conventional_template_path"})
    );
    assert!(
        of(&r, "task").get("template").is_none(),
        "no file, no entry"
    );
    assert_eq!(field(issue, "Kind")["required"], true);
    assert_eq!(
        field(issue, "Kind")["values"]["value"],
        json!(["bug", "chore"])
    );
    assert!(field(issue, "Severity").get("required").is_none());
    assert_eq!(
        field(issue, "Fixes")["pointer"]["targets"]["value"],
        json!(["task"])
    );
    assert_eq!(
        field(issue, "Fixes")["pointer"]["effective_targets"],
        json!(["task"])
    );
    assert!(field(issue, "See-also")["narrative"]["at"].is_string());
    assert_eq!(field(issue, "State")["roles"][0]["role"], "status");
    assert_eq!(field(issue, "Opened")["roles"][0]["role"], "date");
    let states = &issue["states"];
    assert_eq!(states["initial"]["value"], json!(["Open"]));
    assert_eq!(
        states["transitions"][0]["from"], "Open",
        "declaration order"
    );
    assert_eq!(states["failure"]["value"], json!(["Dropped"]));
    let require = &issue["conditions"][0];
    assert_eq!(require["kind"], "require");
    assert_eq!(require["when"], json!(["Closed"]));
    assert_eq!(require["when_field"]["Kind"], json!(["bug"]));
    assert_eq!(
        require["sentence"],
        "when State is Closed and Kind is bug, Severity is required"
    );
    assert_eq!(
        issue["conditions"][1]["sentence"],
        "when State is Open, Fixes is forbidden"
    );
    assert_eq!(issue["gates"][0]["to"], json!(["Dropped"]));
    assert_eq!(issue["gates"][0]["logins"], json!(["someone"]));
    assert!(issue["revision_log"]["at"].is_string());
    assert_eq!(of(&r, "note")["gates"][0]["paths"], json!(["docs/notes"]));
    let task = of(&r, "task");
    let pair = &task["relation_pairs"][0];
    assert_eq!(pair["target_types"], json!(["task"]));
    assert_eq!(pair["reciprocity"], "none");
    assert_eq!(
        task["prefix"],
        json!({"value": "TASK", "default": true, "source": "type_name_uppercased"})
    );
    assert_eq!(
        task["states"]["initial"],
        json!({"required_of_records": false, "new_writes": "Todo", "default": true, "source": "first_status_value"})
    );
    let gates = &r["repository"]["authority"];
    assert!(
        gates["at"].is_string() && gates["value"]["gates"].is_array(),
        "{r}"
    );
}

#[test]
fn rules_set_on_by_declaration_and_declared_but_not_enabled() {
    let dir = repo("rules", FIXTURE);
    let (_, r) = run(&dir, &["explain", "--types", "--full"]);
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
    assert_eq!(values["source"], "default_on");
    assert!(values.get("at").is_none());
    assert_eq!(
        values["declared_by"][0],
        "record_types.issue.field_values:11"
    );
    let required = on
        .iter()
        .find(|e| e["id"] == "status.required-field-missing")
        .unwrap();
    assert_eq!(
        required["declared_by"][0],
        "record_types.issue.statuses.require:25"
    );
    let off = rules["declared_but_not_enabled"].as_array().unwrap();
    let undeclared = off
        .iter()
        .find(|e| e["id"] == "status.undeclared-value")
        .unwrap();
    assert_eq!(
        undeclared["declared_by"][0],
        "record_types.issue.statuses.values:18"
    );
    assert!(off.iter().all(|e| e["id"] != "status.transition-allowed"));
}

#[test]
fn one_type_and_an_unknown_type() {
    let dir = repo("one", FIXTURE);
    let (out, r) = run(&dir, &["explain", "--type", "task"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(r["types"].as_array().unwrap().len(), 1);
    assert!(
        r["types"][0]["header_fields"][0]["at"].is_string(),
        "full detail: {r}"
    );
    let (out, r) = run(&dir, &["explain", "--type", "nope"]);
    assert_eq!(out.status.code(), Some(2), "{r}");
    assert_eq!(r["status"], "not-run");
    let message = r["error"].as_str().unwrap();
    assert!(message.contains("issue, note, task"), "{message}");
}

#[test]
fn one_type_keeps_only_the_rules_that_act_on_it() {
    let dir = repo("scoped", FIXTURE);
    let (_, r) = run(&dir, &["explain", "--type", "note"]);
    let set: Vec<&Value> = r["rules"]["set"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| &e["id"])
        .collect();
    assert_eq!(set, ["header.required-fields"], "{r}");
    assert!(r["rules"].get("on_by_declaration").is_none(), "{r}");
    assert!(r["next_action"].is_null());
}

#[test]
fn the_summary_is_compact_and_points_at_the_detail() {
    let dir = repo("summary", FIXTURE);
    let (_, r) = run(&dir, &["explain", "--types"]);
    let issue = of(&r, "issue");
    assert_eq!(issue["prefix"], "ISS");
    assert_eq!(issue["at"], "record_types.issue:3");
    assert_eq!(issue["dirs"], json!(["docs/issues", "docs/issues-archive"]));
    assert_eq!(issue["required"], json!(["State", "Kind", "Stable-Id"]));
    assert_eq!(issue["references"]["Fixes"], json!(["task"]));
    assert_eq!(issue["states"]["initial"], json!(["Open"]));
    assert_eq!(issue["conditions"], 2);
    assert_eq!(
        of(&r, "task")["prefix"],
        json!({"value": "TASK", "default": true, "source": "type_name_uppercased"})
    );
    assert_eq!(
        r["rules"]["on_by_declaration"]["field.value-not-declared"],
        "error"
    );
    assert_eq!(
        r["rules"]["declared_but_not_enabled"]["status.undeclared-value"],
        "off"
    );
    assert_eq!(
        r["rules"]["on_by_declaration"]["status.required-field-missing"],
        "error"
    );
    assert!(r["rules"].get("set").is_none());
    assert_eq!(r["next_action"], "urzua explain --type <name>");
    assert!(r.get("repository").is_none());
}

fn bytes(dir: &Path, args: &[&str]) -> usize {
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
        .stdout
        .len()
}

#[test]
fn the_reports_stay_under_their_sizes() {
    let dir = repo("size", FIXTURE);
    let summary = bytes(&dir, &["explain", "--types"]);
    assert!(
        summary < MAX_SUMMARY_BYTES,
        "{summary} bytes: three-type summary"
    );
    let full = bytes(&dir, &["explain", "--types", "--full"]);
    assert!(
        full < MAX_FULL_BYTES,
        "{full} bytes: three-type full report"
    );
    let own = bytes(&own_root(), &["explain", "--types"]);
    assert!(
        own < MAX_OWN_SUMMARY_BYTES,
        "{own} bytes: this repository's summary"
    );
}

#[test]
fn a_bare_type_prints_nothing_it_does_not_declare() {
    let dir = repo(
        "bare",
        "schema_version: 2\nrecord_types:\n  note:\n    dir: docs/notes\n",
    );
    let (_, r) = run(&dir, &["explain", "--types", "--full"]);
    let note = of(&r, "note");
    let mut keys: Vec<&String> = note.as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(keys, ["at", "create", "dirs", "name", "prefix"], "{note}");
    assert_eq!(note["prefix"]["default"], true);
    assert!(r.get("rules").is_none(), "an empty object is omitted: {r}");
    let repo_keys: Vec<&String> = r["repository"].as_object().unwrap().keys().collect();
    assert_eq!(repo_keys, ["schema_version"], "{r}");
}

#[test]
fn this_repository_configuration_loads() {
    let (out, r) = run(&own_root(), &["explain", "--types", "--full"]);
    assert_eq!(out.status.code(), Some(0), "{r}");
    for name in ["adr", "spec", "milestone", "bug"] {
        assert!(of(&r, name)["dirs"][0]["at"].is_string());
    }
    assert_positions(&r);
}

/// The keys a configuration struct accepts, read from the load error an
/// unknown key produces (`expected one of ...`), so a key added to the struct
/// is listed here without editing this test.
fn accepted_keys(dir: &Path, config: &str) -> Vec<String> {
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    let (_, r) = run(dir, &["explain", "--types"]);
    let error = r["error"]
        .as_str()
        .unwrap_or_else(|| panic!("no load error: {r}"));
    let list = error
        .split("expected one of")
        .nth(1)
        .unwrap_or_else(|| panic!("{error}"));
    list.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// Whether the full report shows a per-type key: through its own rendering,
/// or raw under its own name.
fn type_key_shown(t: &Value, key: &str) -> bool {
    let fields = t["header_fields"].as_array().cloned().unwrap_or_default();
    let any_field = |k: &str| fields.iter().any(|f| f.get(k).is_some());
    match key {
        "dir" | "extra_dirs" => t["dirs"].as_array().is_some_and(|d| d.len() > 1),
        "required_fields" => any_field("required"),
        "known_fields" => fields.iter().any(|f| f.get("required").is_none()),
        "field_values" => any_field("values"),
        "pointer_fields" => any_field("pointer"),
        "narrative_fields" => any_field("narrative"),
        "relation_fields" => any_field("roles"),
        "statuses" => t["states"].is_object(),
        other => t[other]["at"].is_string(),
    }
}

#[test]
fn every_key_a_record_type_accepts_is_reported_when_declared() {
    let dir = repo("every-key", EVERY_KEY);
    let type_keys = accepted_keys(
        &dir,
        "schema_version: 2\nrecord_types:\n  n:\n    dir: docs/n\n    zz_unknown: 1\n",
    );
    let status_keys = accepted_keys(
        &dir,
        "schema_version: 2\nrecord_types:\n  n:\n    dir: docs/n\n    statuses: {values: [A], zz_unknown: 1}\n",
    );
    assert!(
        type_keys.len() > 20 && status_keys.len() > 10,
        "{type_keys:?} {status_keys:?}"
    );
    std::fs::write(dir.join(".urzua/config.yaml"), EVERY_KEY).unwrap();
    let (out, r) = run(&dir, &["explain", "--types", "--full"]);
    assert_eq!(out.status.code(), Some(0), "{r}");
    assert_positions(&r);
    let types = r["types"].as_array().unwrap();
    for key in &type_keys {
        assert!(
            types.iter().any(|t| type_key_shown(t, key)),
            "per-type key `{key}` is not in the report: declare it in every_key.yaml, and render it"
        );
    }
    let work = of(&r, "work");
    for key in &status_keys {
        let shown = match key.as_str() {
            "require" | "forbid" => work["conditions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["kind"] == key.as_str()),
            other => work["states"][other].is_object() || work["states"][other].is_array(),
        };
        assert!(
            shown,
            "statuses key `{key}` is not in the report: {}",
            work["states"]
        );
    }
    let pair = &work["relation_pairs"][0];
    for key in [
        "name",
        "forward",
        "inverse",
        "reciprocity",
        "cross_type",
        "acyclic",
        "at",
    ] {
        assert!(pair.get(key).is_some(), "pair key `{key}`: {pair}");
    }
}

#[test]
fn a_requirement_names_what_counts_as_missing() {
    let dir = repo("missing", EVERY_KEY);
    let (_, r) = run(&dir, &["explain", "--type", "work"]);
    let work = of(&r, "work");
    let require = &work["conditions"][0];
    assert_eq!(require["kind"], "require");
    let missing = &require["counts_as_missing"];
    assert_eq!(missing["pending"]["value"], json!(["pending"]));
    assert_eq!(missing["placeholders"]["value"], json!(["tbd"]));
    assert!(missing["pending"]["at"].is_string());
    assert!(work["conditions"][1].get("counts_as_missing").is_none());
}

#[test]
fn every_top_level_key_but_types_and_rules_is_under_repository() {
    let dir = repo("top-level", EVERY_KEY);
    let keys = accepted_keys(
        &dir,
        "schema_version: 2\nzz_unknown: 1\nrecord_types:\n  n:\n    dir: docs/n\n",
    );
    assert!(keys.len() > 10, "{keys:?}");
    let (_, r) = run(&own_root(), &["explain", "--types", "--full"]);
    let repository = r["repository"].as_object().unwrap();
    for key in keys
        .iter()
        .filter(|k| *k != "record_types" && *k != "rules")
    {
        assert!(
            repository.get(key).is_some_and(|e| e["at"].is_string()),
            "top-level key `{key}` is not under `repository`; this repository's configuration declares every one"
        );
    }
}

const ITEMS: &str = "schema_version: 2
record_types:
  work:
    dir: docs/work
    required_fields: [Status, Stable-Id]
    known_fields: [Realized-by]
    statuses:
      values: [Planned, Doing, Done]
      success: [Done]
      delivery:
        delivered_tiers: [Implemented]
        unevidenced_tiers: [Not started]
        not_done: [Planned, Doing]
        set_status: Done
        items:
          section: Done means
          clause_separator: \" - \"
          evidence_marker: \"evidence:\"
          waiver_marker: \"waived:\"
          partial_status: Doing
    sections: {from: headings, depth: 2, required: [{name: Done means}]}
";

#[test]
fn rules_on_by_a_status_declaration_are_shown_in_effect_for_the_type() {
    let dir = repo("items", ITEMS);
    std::fs::create_dir_all(dir.join("docs/work")).unwrap();
    let (out, r) = run(&dir, &["explain", "--type", "work"]);
    assert_eq!(out.status.code(), Some(0), "{r}");
    let on = r["rules"]["on_by_declaration"].as_array().unwrap();
    for id in [
        "status.delivery-agreement",
        "status.done-items-shape",
        "status.waived-item-stale",
    ] {
        let rule = on
            .iter()
            .find(|e| e["id"] == id)
            .unwrap_or_else(|| panic!("{id} not in effect: {r}"));
        assert_eq!(rule["default"], true);
        let by: Vec<&str> = rule["declared_by"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(
            by.iter()
                .any(|b| b.starts_with("record_types.work.statuses.delivery")),
            "{id}: {by:?}"
        );
    }
    assert!(
        r["rules"].get("set").is_none(),
        "rules: names none of them: {r}"
    );
}

fn write_repo(name: &str, config: &str) -> common::TempDir {
    let dir = repo(name, config);
    for d in ["docs/a", "docs/b", "docs/note"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    dir
}

fn rule_ids(r: &Value, list: &str) -> Vec<String> {
    r["rules"][list]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|e| e["id"].as_str().unwrap().to_string())
                .collect()
        })
        .unwrap_or_default()
}

const NOTE_EXPIRY: &str = "schema_version: 2
record_types:
  note:
    dir: docs/note
    required_fields: [Status, Last-audited]
    known_fields: [Realized-by]
    statuses: {values: [Open, Done]}
rules:
  embodiment.claim-expired:
    level: LEVEL
    anchor_field: Last-audited
    periods: [{type: note, status: [Done], days: 45}]
";

#[test]
fn expiry_is_shown_only_while_its_rule_is_in_effect_and_the_rule_is_listed() {
    let off = write_repo("expiry-off", &NOTE_EXPIRY.replace("LEVEL", "off"));
    let (_, r) = run(&off, &["explain", "--type", "note"]);
    assert!(of(&r, "note")["states"].get("expiry").is_none(), "{r}");
    let on = write_repo("expiry-on", &NOTE_EXPIRY.replace("LEVEL", "warn"));
    let (_, r) = run(&on, &["explain", "--type", "note"]);
    assert_eq!(of(&r, "note")["states"]["expiry"][0]["days"], 45, "{r}");
    assert!(
        rule_ids(&r, "set").contains(&"embodiment.claim-expired".to_string()),
        "{r}"
    );
}

#[test]
fn one_type_lists_rules_declared_outside_its_own_keys() {
    let (_, r) = run(&own_root(), &["explain", "--type", "adr"]);
    assert!(
        rule_ids(&r, "set").contains(&"diff.decider-required".to_string()),
        "an authority gate on the type: {}",
        r["rules"]
    );
    let (_, r) = run(&own_root(), &["explain", "--type", "milestone"]);
    let set = rule_ids(&r, "set");
    for id in [
        "relation.pair-reciprocity",
        "sequence.cycle",
        "sequence.unresolved-blocker",
        "sequence.status-agreement",
        "embodiment.claim-expired",
    ] {
        assert!(set.contains(&id.to_string()), "{id}: {set:?}");
    }
}

#[test]
fn every_rule_that_runs_for_a_type_is_listed_for_it() {
    // This repository's configuration sets most rules; any rule it sets that
    // reads records, or reads a declaration a type makes, must be listed for
    // some type, or the per-type view hides a rule that judges that type.
    let (_, full) = run(&own_root(), &["explain", "--types", "--full"]);
    let names: Vec<String> = full["types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    let mut listed = std::collections::BTreeSet::new();
    for name in &names {
        let (_, r) = run(&own_root(), &["explain", "--type", name]);
        listed.extend(rule_ids(&r, "set"));
    }
    let (_, rules) = run(&own_root(), &["rules"]);
    let per_record: Vec<String> = rules["rules"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["scope"] == "records")
        .map(|d| d["id"].as_str().unwrap().to_string())
        .collect();
    assert!(per_record.len() > 20, "{rules}");
    let set: Vec<String> = full["rules"]["set"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["level"] != "off")
        .map(|e| e["id"].as_str().unwrap().to_string())
        .collect();
    for id in set.iter().filter(|id| per_record.contains(id)) {
        assert!(
            listed.contains(id),
            "{id} judges records but no `--type` lists it"
        );
    }
    assert!(listed.contains("diff.decider-required"), "{listed:?}");
}

const NARROWED: &str = "schema_version: 2
record_types:
  a:
    dir: docs/a
    required_fields: [Status]
    known_fields: [Up, Down, Other]
    pointer_fields:
      - {name: Up, targets: [a, b]}
      - Other
    narrative_fields: []
  b:
    dir: docs/b
    required_fields: [Status]
    known_fields: [Down]
relation_pairs:
  - name: tree
    forward: Up
    inverse: Down
    reciprocity: none
    target_types: [b]
";

#[test]
fn a_narrowed_reference_names_each_declaration_and_the_result() {
    let dir = write_repo("narrowed", NARROWED);
    let (out, r) = run(&dir, &["explain", "--type", "a"]);
    assert_eq!(out.status.code(), Some(0), "{r}");
    let up = &field(of(&r, "a"), "Up")["pointer"];
    assert_eq!(up["targets"]["value"], json!(["a", "b"]));
    assert_eq!(
        up["targets"]["at"],
        "record_types.a.pointer_fields[0].targets:8"
    );
    assert_eq!(up["pair_target_types"][0]["value"], json!(["b"]));
    assert_eq!(
        up["pair_target_types"][0]["at"],
        "relation_pairs[0].target_types:20"
    );
    assert_eq!(up["effective_targets"], json!(["b"]));
    let other = &field(of(&r, "a"), "Other")["pointer"];
    assert!(
        other.get("targets").is_none(),
        "no line that declares none: {other}"
    );
    assert_eq!(other["effective_targets"], "any");
}

#[test]
fn an_unconstrained_reference_says_any_type() {
    let dir = write_repo("any", NARROWED);
    let (_, r) = run(&dir, &["explain", "--types"]);
    assert_eq!(of(&r, "a")["references"]["Other"], "any");
    assert_eq!(of(&r, "a")["references"]["Up"], json!(["b"]));
}

#[test]
fn an_undeclared_initial_state_requires_no_start_status() {
    let dir = repo("initial", FIXTURE);
    let (_, r) = run(&dir, &["explain", "--type", "task"]);
    let initial = &of(&r, "task")["states"]["initial"];
    assert_eq!(initial["required_of_records"], false);
    assert_eq!(initial["new_writes"], "Todo");
    assert_eq!(initial["default"], true);
    assert!(initial.get("value").is_none(), "{initial}");
}

#[test]
fn the_summary_marks_its_defaults() {
    let dir = repo("summary-defaults", FIXTURE);
    let (_, r) = run(&dir, &["explain", "--types"]);
    let task = of(&r, "task");
    assert_eq!(task["prefix"]["default"], true);
    assert_eq!(task["states"]["initial"]["default"], true);
    assert_eq!(
        of(&r, "issue")["prefix"],
        "ISS",
        "a declared prefix stays plain"
    );
}

#[test]
fn no_record_type_is_an_empty_list_and_a_plain_error() {
    let dir = repo("empty", "schema_version: 2\nrecord_types: {}\n");
    let (out, r) = run(&dir, &["explain", "--types"]);
    assert_eq!(out.status.code(), Some(0), "{r}");
    assert_eq!(r["types"], json!([]));
    let (_, r) = run(&dir, &["explain", "--type", "x"]);
    let error = r["error"].as_str().unwrap();
    assert!(error.ends_with("declares no record type"), "{error}");
}

#[test]
fn full_goes_with_types_only() {
    let dir = repo("full-type", FIXTURE);
    let (out, r) = run(&dir, &["explain", "--type", "task", "--full"]);
    assert_eq!(out.status.code(), Some(2), "{r}");
    assert!(r["error"].as_str().unwrap().contains("--full"), "{r}");
}

#[test]
fn conditions_whose_rule_is_off_are_counted_apart() {
    let config = format!("{FIXTURE}  status.forbidden-field-present: off\n");
    let dir = repo("conditions-off", &config);
    let (_, r) = run(&dir, &["explain", "--types"]);
    let issue = of(&r, "issue");
    assert_eq!(issue["conditions"], 1, "{issue}");
    assert_eq!(issue["conditions_not_enabled"], 1, "{issue}");
    let (_, r) = run(&dir, &["explain", "--type", "issue"]);
    let forbid = &of(&r, "issue")["conditions"][1];
    assert_eq!(forbid["rule"]["level"], "off", "{forbid}");
}

#[test]
fn an_aliased_declaration_cites_where_it_is_defined() {
    let config = "schema_version: 2
record_types:
  a:
    dir: docs/a
    required_fields: [Status]
    statuses: &st
      values: [Open, Done]
  b:
    dir: docs/b
    required_fields: [Status]
    statuses: *st
";
    let dir = write_repo("alias", config);
    let (_, r) = run(&dir, &["explain", "--type", "b"]);
    assert_eq!(
        of(&r, "b")["aliases"],
        json!([{"at": "record_types.b.statuses:11", "defined_at": "record_types.a.statuses:6"}])
    );
}

#[test]
fn built_in_pending_words_apply_when_none_are_declared() {
    let dir = repo("built-in-pending", FIXTURE);
    let (_, r) = run(&dir, &["explain", "--type", "issue"]);
    let missing = &of(&r, "issue")["conditions"][0]["counts_as_missing"];
    assert_eq!(
        missing["pending"],
        json!({"default": true, "source": "built_in_list"})
    );
    assert_eq!(
        missing["placeholders"],
        json!({"default": true, "source": "built_in_list"})
    );
}
