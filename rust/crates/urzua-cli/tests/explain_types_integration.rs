//! `urzua explain --types [--full]` and `urzua explain --type <name>`: what the configuration declares for its record types, each
//! entry with the key path and line it was read from.

mod common;

use std::path::Path;
use std::process::Command;

use serde_json::{json, Value};

const FIXTURE: &str = include_str!("fixtures/explain_types/config.yaml");
const EVERY_KEY: &str = include_str!("fixtures/explain_types/every_key.yaml");

/// The summary of the three-type fixture, as printed, stays under this many bytes.
const MAX_SUMMARY_BYTES: usize = 1_900;
/// The summary of the fixture declaring every added key stays under this many bytes.
const MAX_CHECKLIST_SUMMARY_BYTES: usize = 1_500;
/// The full report of the three-type fixture stays under this many bytes.
const MAX_FULL_BYTES: usize = 10_800;
/// The summary of this repository's own configuration stays under this many bytes.
const MAX_OWN_SUMMARY_BYTES: usize = 6_200;

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
    assert_eq!(issue["create_command"], "urzua new issue '<title>'");
    assert_eq!(issue["prefix"]["value"], "ISS");
    assert_eq!(issue["prefix"]["at"], "record_types.issue.prefix:6");
    assert_eq!(issue["dirs"][1]["value"], "docs/issues-archive");
    assert_eq!(issue["header_shape"]["value"], "yaml-frontmatter");
    assert_eq!(
        issue["template"],
        json!({"value": ".urzua/templates/issue.md", "default": true, "source": "conventional_template_path", "exists_when_run": true})
    );
    assert_eq!(
        of(&r, "task")["template"],
        json!({"value": ".urzua/templates/task.md", "default": true, "source": "conventional_template_path", "exists_when_run": false}),
        "the path is printed with the fact that no file is there"
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
    assert!(
        task["states"].get("initial").is_none(),
        "an undeclared initial is not printed"
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
    assert_eq!(
        issue["conditions"],
        json!([
            "declared: when State is Closed and Kind is bug, Severity is required",
            "declared: when State is Open, Fixes is forbidden"
        ])
    );
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
    let declared = repo("size-declared", CHECKLIST);
    let all = bytes(&declared, &["explain", "--types"]);
    assert!(
        all < MAX_CHECKLIST_SUMMARY_BYTES,
        "{all} bytes: summary with gates, values, conditions, classes and a checklist"
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
        ["at", "create_command", "dirs", "name", "prefix", "template"],
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
    assert_eq!(up["pair_target_types"][0]["pair"], "tree");
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
    assert!(of(&r, "task")["states"].get("initial").is_none());
    assert_eq!(
        of(&r, "task")["states"]["new_writes"],
        json!({"value": "Todo", "default": true, "source": "urzua_new_scaffold"})
    );
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
    assert_eq!(s["path_gates"][0]["paths"], json!(["docs/notes"]), "{s}");
    assert!(
        s["path_gates"].is_array(),
        "the summary lists the gates without the note"
    );
    let (_, f) = run(&dir, &["explain", "--types", "--full"]);
    assert!(f["path_gates"]["note"]
        .as_str()
        .unwrap()
        .contains("declares who may change"));
    let (_, r) = run(&dir, &["explain", "--type", "issue"]);
    assert_eq!(of(&r, "issue")["gates"][0]["to"], json!(["Dropped"]));
    assert_eq!(of(&r, "issue")["gates"].as_array().unwrap().len(), 1);
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

fn pattern_repo(name: &str, number: &str, slug: &str, existing: &[&str]) -> common::TempDir {
    let config = format!(
        "schema_version: 2
record_types:
  t:
    dir: docs/t
    prefix: T
    identity: {{from: filename, pattern: '^(?P<prefix>T)-(?P<number>{number})-(?P<slug>{slug})$'}}
    required_fields: [Status]
"
    );
    let dir = write_repo(name, &config);
    std::fs::create_dir_all(dir.join("docs/t")).unwrap();
    for f in existing {
        std::fs::write(dir.join("docs/t").join(f), "---\nStatus: x\n---\n# x\n").unwrap();
    }
    dir
}

#[test]
fn urzua_new_fits_an_identity_pattern_to_the_real_number_and_title() {
    let rows: [(&str, &str, &[&str], &str, &str); 6] = [
        (
            r"[1-9]\d",
            ".+",
            &["T-10-a.md", "T-11-b.md"],
            "x",
            "T-12-x.md",
        ),
        (r"[1-9]\d{3}", ".+", &["T-1000-a.md"], "x", "T-1001-x.md"),
        (r"\d+", r"\d+", &[], "2024", "T-1-2024.md"),
        (
            r"\d+",
            "[a-z]+-[a-z]+",
            &[],
            "Hello World",
            "T-1-hello-world.md",
        ),
        (r"\d+", "[a-z-]{1,4}", &[], "Hi", "T-1-hi.md"),
        (
            r"\d+",
            "[a-z-]{6,}",
            &[],
            "Hello World",
            "T-1-hello-world.md",
        ),
    ];
    for (i, (number, slug, existing, title, expected)) in rows.iter().enumerate() {
        let dir = pattern_repo(&format!("fit-{i}"), number, slug, existing);
        let (_, r) = run(&dir, &["explain", "--type", "t"]);
        assert!(of(&r, "t")["create_command"].is_string(), "{r}");
        let (out, created) = run(&dir, &["new", "t", title, "--by", "someone"]);
        assert_eq!(out.status.code(), Some(0), "row {i}: {created}");
        assert!(
            dir.join("docs/t").join(expected).is_file(),
            "row {i}: expected {expected}: {created}"
        );
    }
}

#[test]
fn an_unfit_pattern_shows_a_command_and_urzua_new_says_why() {
    let config = "schema_version: 2
record_types:
  b:
    dir: docs/b
    prefix: B
    identity: {from: filename, pattern: '^(?P<prefix>X)-(?P<number>\\d+)-(?P<slug>.+)$'}
    required_fields: [Status]
";
    let dir = write_repo("unfit", config);
    let (_, r) = run(&dir, &["explain", "--type", "b"]);
    assert_eq!(of(&r, "b")["create_command"], "urzua new b '<title>'");
    let (out, refused) = run(&dir, &["new", "b", "hello", "--by", "someone"]);
    assert_eq!(out.status.code(), Some(2), "{refused}");
    assert!(
        refused["error"].as_str().unwrap().contains("does not read"),
        "{refused}"
    );
}

#[test]
fn the_create_command_is_shell_quoted() {
    let config = "schema_version: 2\nrecord_types:\n  my type:\n    dir: docs/a\n    prefix: MT\n";
    let dir = write_repo("quoted", config);
    let (out, r) = run(&dir, &["explain", "--type", "my type"]);
    assert_eq!(out.status.code(), Some(0), "{r}");
    assert_eq!(
        of(&r, "my type")["create_command"],
        "urzua new 'my type' '<title>'"
    );
}

#[test]
fn a_pair_field_names_its_pair_as_declared() {
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
relation_pairs:
  - {name: tree, forward: Up, inverse: Down, reciprocity: none}
";
    let dir = write_repo("pair-field", config);
    let (_, r) = run(&dir, &["explain", "--type", "b"]);
    let down = &field(of(&r, "b"), "Down")["relation_pairs"][0];
    assert_eq!(down["name"], "tree", "{r}");
    assert_eq!(down["forward"], "Up");
    assert_eq!(down["inverse"], "Down");
    assert_eq!(down["at"], "relation_pairs[0]:12");
    assert!(down.get("target_types").is_none());
}

const CHECKLIST: &str = include_str!("fixtures/explain_types/checklist.yaml");

fn line_of(config: &str, needle: &str) -> usize {
    config
        .lines()
        .position(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("no line holds {needle}"))
        + 1
}

fn summary_of(dir: &Path) -> Value {
    run(dir, &["explain", "--types"]).1
}

fn detail_of(dir: &Path, name: &str) -> Value {
    of(&run(dir, &["explain", "--type", name]).1, name).clone()
}

#[test]
fn the_summary_carries_what_a_type_declares_to_gate_constrain_and_log() {
    let dir = repo("summary-additions", CHECKLIST);
    let task = of(&summary_of(&dir), "task").clone();
    assert_eq!(
        task["gated"],
        json!({"Done": ["owners", "leads"], "WontDo": "leads"}),
        "a status two gates name lists both groups, in declaration order; no logins"
    );
    assert_eq!(task["field_values"], json!({"Kind": ["feature", "fix"]}));
    assert_eq!(
        task["conditions"],
        json!(["declared: when Status is Done and Kind is fix, Author is required"])
    );
    assert_eq!(task["revision_log_classes"], json!(["major", "minor"]));
    assert_eq!(
        task["checklist"],
        json!({
            "section": "Acceptance",
            "evidence": {"code": "Implemented", "record": "Decided", "spec": "Specified", "test": "Verified"},
            "resolving_tiers": ["Implemented", "Verified", "Decided"]
        }),
        "every category has a declared tier, so none is listed as without one"
    );
}

#[test]
fn a_type_that_declares_none_of_it_has_none_of_the_keys() {
    let dir = repo("summary-absent", CHECKLIST);
    let note = of(&summary_of(&dir), "note").clone();
    let keys: Vec<&String> = note.as_object().unwrap().keys().collect();
    for absent in [
        "gated",
        "field_values",
        "conditions",
        "revision_log_classes",
        "checklist",
    ] {
        assert!(!keys.iter().any(|k| *k == absent), "{absent} in {note}");
    }
    let dir = repo("summary-absent-fixture", FIXTURE);
    let s = summary_of(&dir);
    let issue = of(&s, "issue");
    assert!(
        issue.get("revision_log_classes").is_none(),
        "a revision_log that declares no classes adds no key: {issue}"
    );
    assert!(issue.get("checklist").is_none(), "{issue}");
    let task = of(&s, "task");
    assert!(task.get("gated").is_none() && task.get("field_values").is_none());
    assert!(task.get("conditions").is_none(), "no count either: {task}");
}

#[test]
fn the_summary_values_are_the_detail_values() {
    let dir = repo("one-source", CHECKLIST);
    let s = summary_of(&dir);
    let summary = of(&s, "task");
    let detail = detail_of(&dir, "task");
    for f in detail["header_fields"].as_array().unwrap() {
        if let Some(values) = f.get("values") {
            assert_eq!(
                summary["field_values"][f["name"].as_str().unwrap()],
                values["value"]
            );
        }
    }
    let sentences: Vec<&Value> = detail["conditions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| &c["sentence"])
        .collect();
    assert_eq!(
        summary["conditions"]
            .as_array()
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        sentences
    );
    for g in detail["gates"].as_array().unwrap() {
        for status in g["to"].as_array().unwrap() {
            let shown = &summary["gated"][status.as_str().unwrap()];
            let by = &g["by"];
            assert!(
                shown == by || shown.as_array().is_some_and(|a| a.contains(by)),
                "{status}: {shown} vs {by}"
            );
        }
    }
    assert_eq!(
        summary["revision_log_classes"],
        detail["revision_log"]["value"]["classes"]
    );
    let checklist = &detail["checklist"];
    assert_eq!(
        summary["checklist"]["section"],
        checklist["section"]["value"]
    );
    assert_eq!(
        summary["checklist"]["resolving_tiers"],
        checklist["resolving_tiers"]["value"]
    );
    for row in checklist["evidence_tiers"].as_array().unwrap() {
        let category = row["category"].as_str().unwrap();
        match row.get("tier") {
            Some(tier) => assert_eq!(summary["checklist"]["evidence"][category], *tier),
            None => assert!(summary["checklist"]["no_tier_declares"]
                .as_array()
                .unwrap()
                .contains(&row["category"])),
        }
    }
}

#[test]
fn a_checklist_names_its_categories_tiers_and_where_each_is_declared() {
    let dir = repo("checklist-detail", CHECKLIST);
    let c = detail_of(&dir, "task")["checklist"].clone();
    assert_positions(&c);
    assert_eq!(
        c["section"]["at"],
        format!(
            "record_types.task.statuses.delivery.items.section:{}",
            line_of(CHECKLIST, "section: Acceptance")
        )
    );
    assert_eq!(c["evidence_marker"]["value"], "evidence:");
    assert_eq!(
        c["evidence_categories"],
        json!({"value": ["spec", "code", "test", "record"], "source": "engine_locator_categories"})
    );
    let rows = c["evidence_tiers"].as_array().unwrap();
    let row = |category: &str| rows.iter().find(|r| r["category"] == category).unwrap();
    assert_eq!(
        row("code"),
        &json!({
            "category": "code",
            "tier": "Implemented",
            "in_resolving_tiers": true,
            "at": format!("embodiment.tiers[1]:{}", line_of(CHECKLIST, "evidence: code"))
        })
    );
    assert_eq!(row("record")["tier"], "Decided");
    assert_eq!(
        row("spec")["in_resolving_tiers"],
        false,
        "a tier the type does not list as resolving"
    );
    assert_eq!(
        c["resolving_tiers"]["at"],
        format!(
            "record_types.task.statuses.delivery.items.resolving_tiers:{}",
            line_of(CHECKLIST, "resolving_tiers:")
        )
    );
    assert_eq!(
        c["resolving_tiers"]["value"],
        json!(["Implemented", "Verified", "Decided"])
    );
    let form = c["locator_form"]["value"].as_str().unwrap();
    assert!(form.contains("`<category>:<locator>`") && form.contains("path[#scope][@anchor]"));
    assert_eq!(c["locator_form"]["source"], "engine_locator_grammar");
}

#[test]
fn without_declared_tiers_or_resolving_tiers_the_engines_are_shown_as_defaults() {
    let config = CHECKLIST
        .replace(
            "          resolving_tiers: [Implemented, Verified, Decided]\n",
            "",
        )
        .split("embodiment:")
        .next()
        .unwrap()
        .to_string()
        + "authority:\n  deciders:\n    owners: {logins: [someone]}\n";
    let dir = repo("checklist-defaults", &config);
    let c = detail_of(&dir, "task")["checklist"].clone();
    assert_positions(&c);
    assert_eq!(
        c["resolving_tiers"],
        json!({"value": ["Implemented", "Verified"], "default": true, "source": "delivered_tiers"})
    );
    let rows = c["evidence_tiers"].as_array().unwrap();
    let code = rows.iter().find(|r| r["category"] == "code").unwrap();
    assert_eq!(code["default"], true);
    assert_eq!(code["source"], "built_in_embodiment_tiers");
    assert!(code.get("at").is_none());
    let record = rows.iter().find(|r| r["category"] == "record").unwrap();
    assert_eq!(record["no_tier_declares_it"], true);
    let s = summary_of(&dir);
    let marker = &of(&s, "task")["checklist"];
    assert_eq!(
        marker["resolving_tiers"],
        json!({"value": ["Implemented", "Verified"], "default": true, "source": "delivered_tiers"})
    );
    assert_eq!(marker["evidence"]["default"], true);
    assert_eq!(marker["no_tier_declares"], json!(["record"]));
}

#[test]
fn the_summary_puts_scalar_arrays_on_one_line_and_the_other_outputs_do_not() {
    let dir = repo("format", FIXTURE);
    let text = |args: &[&str]| {
        String::from_utf8(
            common::urzua()
                .args(args)
                .current_dir(&dir)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
    };
    let summary = text(&["explain", "--types"]);
    assert!(
        summary.contains("\"dirs\": [\"docs/issues\", \"docs/issues-archive\"],"),
        "{summary}"
    );
    serde_json::from_str::<Value>(&summary).unwrap();
    assert_eq!(summary, text(&["explain", "--types"]), "deterministic");
    for args in [
        &["explain", "--types", "--full"][..],
        &["explain", "--type", "issue"][..],
    ] {
        let t = text(args);
        assert!(
            t.contains("\"value\": [\n"),
            "{args:?} keeps one element per line"
        );
    }
}

#[test]
fn the_template_says_whether_its_file_exists() {
    let dir = repo("template-exists", FIXTURE);
    std::fs::write(dir.join(".urzua/templates/issue.md"), "# {title}\n").unwrap();
    for (name, exists) in [("issue", true), ("task", false)] {
        let t = detail_of(&dir, name)["template"].clone();
        assert_eq!(t["exists_when_run"], exists, "{t}");
        assert_eq!(t["value"], format!(".urzua/templates/{name}.md"));
        assert_eq!(t["default"], true);
    }
    std::fs::remove_file(dir.join(".urzua/templates/issue.md")).unwrap();
    assert_eq!(
        detail_of(&dir, "issue")["template"]["exists_when_run"],
        false
    );
}
