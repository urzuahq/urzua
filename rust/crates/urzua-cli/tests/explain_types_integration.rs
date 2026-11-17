//! `urzua explain --types [--full]` and `urzua explain --type <name>`: what the configuration declares for its record types, each
//! entry with the key path and line it was read from.

mod common;

use std::path::Path;
use std::process::Command;

use serde_json::{json, Value};

const FIXTURE: &str = include_str!("fixtures/explain_types/config.yaml");
const EVERY_KEY: &str = include_str!("fixtures/explain_types/every_key.yaml");

/// The summary of the three-type fixture, as printed, stays under this many bytes.
const MAX_SUMMARY_BYTES: usize = 2_350;
/// The full report of the three-type fixture stays under this many bytes.
const MAX_FULL_BYTES: usize = 10_000;
/// The summary of this repository's own configuration stays under this many bytes.
const MAX_OWN_SUMMARY_BYTES: usize = 6_050;

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
    assert_eq!(issue["create_command"], "urzua new issue \"<title>\"");
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
        "declared: when State is Closed and Kind is bug, Severity is required"
    );
    assert_eq!(
        issue["conditions"][1]["sentence"],
        "declared: when State is Open, Fixes is forbidden"
    );
    assert_eq!(issue["gates"][0]["to"], json!(["Dropped"]));
    assert_eq!(issue["gates"][0]["logins"], json!(["someone"]));
    assert!(issue["revision_log"]["at"].is_string());
    assert!(
        of(&r, "note").get("gates").is_none(),
        "a path gate is not attributed to a type"
    );
    assert_eq!(r["path_gates"]["gates"][0]["paths"], json!(["docs/notes"]));
    assert_eq!(r["path_gates"]["gates"][0]["at"], "authority.gates[1]:54");
    let task = of(&r, "task");
    assert!(
        task.get("relation_pairs").is_none(),
        "pairs are shown as declared, once"
    );
    let pair = &r["repository"]["relation_pairs"]["value"][0];
    assert_eq!(pair["target_types"], json!(["task"]));
    assert_eq!(pair["reciprocity"], "none");
    assert_eq!(
        task["prefix"],
        json!({"value": "TASK", "default": true, "source": "type_name_uppercased"})
    );
    assert_eq!(
        task["states"]["initial"],
        json!({"required_of_records": false, "default": true, "source": "no_initial_declared"})
    );
    let gates = &r["repository"]["authority"];
    assert!(
        gates["at"].is_string() && gates["value"]["gates"].is_array(),
        "{r}"
    );
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
    assert!(r.get("rules").is_none(), "the summary names no rule: {r}");
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
    assert_eq!(
        keys,
        ["at", "create_command", "dirs", "name", "prefix"],
        "{note}"
    );
    assert_eq!(note["prefix"]["default"], true);
    assert!(
        r.get("rules_set_in_configuration").is_none(),
        "no rule is set, so the section is omitted: {r}"
    );
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
    let pair = &r["repository"]["relation_pairs"]["value"][0];
    for key in [
        "name",
        "forward",
        "inverse",
        "reciprocity",
        "cross_type",
        "acyclic",
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
fn write_repo(name: &str, config: &str) -> common::TempDir {
    let dir = repo(name, config);
    for d in ["docs/a", "docs/b", "docs/note"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    dir
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
    assert_eq!(
        of(&r, "task")["states"]["new_writes"],
        json!({"value": "Todo", "default": true, "source": "urzua_new_scaffold"})
    );
    assert_eq!(initial["default"], true);
    assert!(initial.get("value").is_none(), "{initial}");
}
const UNREQUIRED_STATUS: &str = "schema_version: 2
record_types:
  note:
    dir: docs/note
    known_fields: [Status]
    statuses: {values: [P, Q]}
  task:
    dir: docs/a
    required_fields: [Status]
    statuses: {values: [P, Q], initial: [Q], transitions: {Q: [P]}}
";

fn scaffolded_status(dir: &Path, t: &str) -> Option<String> {
    let (out, r) = run(dir, &["new", t, "x", "--by", "someone"]);
    assert_eq!(out.status.code(), Some(0), "{r}");
    let path = r["path"]
        .as_str()
        .or(r["file"].as_str())
        .unwrap_or_else(|| panic!("{r}"));
    let text = std::fs::read_to_string(dir.join(path)).unwrap();
    text.lines()
        .find_map(|l| l.strip_prefix("Status:"))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty() && v != "null")
}

#[test]
fn new_writes_is_what_urzua_new_writes() {
    let dir = write_repo("new-writes", UNREQUIRED_STATUS);
    for t in ["note", "task"] {
        let (_, r) = run(&dir, &["explain", "--type", t]);
        let shown = of(&r, t)["states"]["new_writes"]["value"]
            .as_str()
            .map(str::to_string);
        assert_eq!(shown, scaffolded_status(&dir, t), "{t}: {r}");
    }
    let (_, r) = run(&dir, &["explain", "--type", "note"]);
    assert!(of(&r, "note")["states"].get("new_writes").is_none());
}

#[test]
fn an_empty_target_intersection_says_none() {
    let config = NARROWED.replace("{name: Up, targets: [a, b]}", "{name: Up, targets: [a]}");
    let dir = write_repo("none", &config);
    let (_, r) = run(&dir, &["explain", "--type", "a"]);
    let up = &field(of(&r, "a"), "Up")["pointer"]["effective_targets"];
    assert_eq!(up["value"], "none", "{up}");
    assert!(up["note"].is_string());
    let (_, s) = run(&dir, &["explain", "--types"]);
    assert_eq!(of(&s, "a")["references"]["Up"]["value"], "none");
    assert!(!s.to_string().contains("\"Up\":[]"));
}
#[test]
fn the_full_report_lists_the_rules_set_as_written() {
    let dir = repo("rules-set", FIXTURE);
    let (_, r) = run(&dir, &["explain", "--types", "--full"]);
    let section = &r["rules_set_in_configuration"];
    assert_eq!(
        section["rules"],
        json!([
            {"id": "header.required-fields", "level": "error", "at": "rules[\"header.required-fields\"]:57"},
            {"id": "status.transition-allowed", "level": "warn", "at": "rules[\"status.transition-allowed\"]:58"}
        ])
    );
    assert!(section["note"].as_str().unwrap().contains("urzua check"));
    for t in r["types"].as_array().unwrap() {
        assert!(
            t.get("rules").is_none(),
            "no type names a rule as applying: {t}"
        );
    }
    let (_, one) = run(&dir, &["explain", "--type", "issue"]);
    assert!(one.get("rules").is_none(), "{one}");
    assert!(of(&one, "issue")["conditions"][0].get("rule").is_none());
    assert!(
        one["note"].as_str().unwrap().contains("urzua check"),
        "{one}"
    );
}

const NOTE_PERIOD: &str = "schema_version: 2
record_types:
  note:
    dir: docs/note
    required_fields: [Status, Last-audited]
    known_fields: [Realized-by]
    statuses: {values: [Open, Done]}
rules:
  embodiment.claim-expired:
    level: off
    anchor_field: Last-audited
    periods: [{type: note, status: [Done], days: 45}]
";

#[test]
fn a_claim_period_is_shown_as_declared_with_the_level_as_written() {
    let dir = write_repo("period", NOTE_PERIOD);
    let (_, r) = run(&dir, &["explain", "--type", "note"]);
    let expiry = &of(&r, "note")["states"]["expiry"];
    assert_eq!(expiry["periods"][0]["days"], 45, "{r}");
    assert_eq!(
        expiry["periods"][0]["at"],
        "rules[\"embodiment.claim-expired\"].periods[0]:12"
    );
    assert_eq!(expiry["level"]["value"], "off");
}

#[test]
fn a_type_urzua_new_refuses_has_no_create_command() {
    let config = "schema_version: 2
record_types:
  log:
    dir: docs/a
    header_shape: none
    statuses: {values: [Open]}
";
    let dir = write_repo("no-header", config);
    let (_, r) = run(&dir, &["explain", "--type", "log"]);
    let log = of(&r, "log");
    assert!(log.get("create_command").is_none(), "{log}");
    assert!(log["states"].get("new_writes").is_none(), "{log}");
    let (out, refused) = run(&dir, &["new", "log", "x", "--by", "someone"]);
    assert_eq!(out.status.code(), Some(2), "{refused}");
    assert_eq!(
        log["cannot_create"], refused["error"],
        "the engine's own reason"
    );
    let (_, s) = run(&dir, &["explain", "--types"]);
    assert!(of(&s, "log").get("create_command").is_none());
}
#[test]
fn a_pair_forward_field_outside_pointer_fields_shows_the_pairs_targets() {
    let config = "schema_version: 2
record_types:
  a:
    dir: docs/a
    required_fields: [Status]
    known_fields: [Up]
  b:
    dir: docs/b
    required_fields: [Status]
    known_fields: [Down]
  c:
    dir: docs/note
    required_fields: [Status]
relation_pairs:
  - {name: tree, forward: Up, inverse: Down, reciprocity: none, target_types: [c]}
";
    let dir = write_repo("pair-forward", config);
    let (out, r) = run(&dir, &["explain", "--type", "a"]);
    assert_eq!(out.status.code(), Some(0), "{r}");
    let up = &field(of(&r, "a"), "Up")["pointer"];
    assert_eq!(up["pair_target_types"][0]["value"], json!(["c"]), "{up}");
    assert_eq!(
        up["pair_target_types"][0]["at"],
        "relation_pairs[0].target_types:15"
    );
    assert_eq!(up["effective_targets"], json!(["c"]));
    let (_, s) = run(&dir, &["explain", "--types"]);
    assert_eq!(of(&s, "a")["references"]["Up"], json!(["c"]));
    assert!(
        of(&s, "b").get("references").is_none(),
        "b carries only the inverse"
    );
}

#[test]
fn full_alone_and_an_empty_summary_name_no_type_command() {
    let dir = repo("full-alone", FIXTURE);
    let (out, r) = run(&dir, &["explain", "--full"]);
    assert_eq!(out.status.code(), Some(2), "{r}");
    assert_eq!(r["next_action"], "urzua explain --types --full");
    let empty = repo("empty-next", "schema_version: 2\nrecord_types: {}\n");
    let (_, r) = run(&empty, &["explain", "--types"]);
    assert!(r["next_action"].is_null(), "{r}");
}

#[test]
fn path_gates_are_listed_once_as_declared_and_types_carry_only_their_own_gates() {
    let dir = repo("path-gates", FIXTURE);
    let (_, s) = run(&dir, &["explain", "--types"]);
    assert_eq!(
        s["path_gates"]["gates"][0]["paths"],
        json!(["docs/notes"]),
        "{s}"
    );
    assert!(s["path_gates"]["note"]
        .as_str()
        .unwrap()
        .contains("whatever the type"));
    let (_, r) = run(&dir, &["explain", "--type", "issue"]);
    assert_eq!(of(&r, "issue")["gates"][0]["to"], json!(["Dropped"]));
    assert_eq!(of(&r, "issue")["gates"].as_array().unwrap().len(), 1);
}

#[test]
fn a_pattern_urzua_new_cannot_fit_is_refused_ahead_and_a_title_refusal_is_not() {
    let config = "schema_version: 2
record_types:
  b:
    dir: docs/b
    prefix: B
    identity: {from: filename, pattern: '^(?P<prefix>X)-(?P<number>\\d+)-(?P<slug>.+)$'}
    required_fields: [Status]
  a:
    dir: docs/a
    required_fields: [Status]
";
    let dir = write_repo("refusals", config);
    let (_, r) = run(&dir, &["explain", "--types", "--full"]);
    let b = of(&r, "b");
    assert!(b.get("create_command").is_none(), "{b}");
    let (out, refused) = run(&dir, &["new", "b", "hello", "--by", "someone"]);
    assert_eq!(out.status.code(), Some(2), "{refused}");
    assert_eq!(
        b["cannot_create"], refused["error"],
        "the reason `urzua new` gives"
    );
    assert!(refused["error"].as_str().unwrap().contains("does not read"));
    let a = of(&r, "a");
    assert_eq!(a["create_command"], "urzua new a \"<title>\"");
    let (out, refused) = run(&dir, &["new", "a", "!!!", "--by", "someone"]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "a title refusal is decided by `urzua new`: {refused}"
    );
}

#[test]
fn a_multi_line_flow_list_gives_each_item_its_line() {
    let config = "schema_version: 2
record_types:
  a:
    dir: docs/a
    required_fields: [Status]
    known_fields: [One,
      Two,
      Three]
";
    let dir = write_repo("flow-lines", config);
    let (_, r) = run(&dir, &["explain", "--type", "a"]);
    let a = of(&r, "a");
    assert_eq!(field(a, "One")["at"], "record_types.a.known_fields[0]:6");
    assert_eq!(field(a, "Two")["at"], "record_types.a.known_fields[1]:7");
    assert_eq!(field(a, "Three")["at"], "record_types.a.known_fields[2]:8");
}

#[test]
fn the_rules_set_keep_their_written_order() {
    let config = format!("{FIXTURE}  field.quality: warn\n  diff.decider-required: error\n");
    let dir = repo("rules-order", &config);
    let (_, r) = run(&dir, &["explain", "--types", "--full"]);
    let ids: Vec<&str> = r["rules_set_in_configuration"]["rules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        [
            "header.required-fields",
            "status.transition-allowed",
            "field.quality",
            "diff.decider-required"
        ]
    );
}

#[test]
fn the_summary_gives_no_create_command() {
    let dir = repo("summary-no-create", FIXTURE);
    let (_, s) = run(&dir, &["explain", "--types"]);
    for t in s["types"].as_array().unwrap() {
        assert!(
            t.get("create").is_none() && t.get("create_command").is_none(),
            "{t}"
        );
    }
}
