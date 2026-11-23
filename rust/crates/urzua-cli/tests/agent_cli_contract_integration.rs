//! `urzua check`'s report against the published agent CLI contract, vendored under
//! `fixtures/agent-cli-contract/` from a release tag (`MILE-358`, `MILE-359`, `ADR-106`).
//! The check reads the local copy only: a `$ref` is answered from the vendored directory, never from
//! the network. It is a structural check of the schemas' stated requirements, not a general JSON
//! Schema validator; it answers the keywords the contract permits its schemas to use.

mod common;

use serde_json::Value;
use std::path::{Path, PathBuf};

fn contract_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/agent-cli-contract")
}

fn schema_dir() -> PathBuf {
    contract_dir().join("schemas")
}

fn load(name: &str) -> Value {
    let text = std::fs::read_to_string(schema_dir().join(name)).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// The JSON Schema keywords the contract permits its schemas to use; the contract's own `make check`
/// fails on any other. This check answers each of them, and panics on anything else.
const CONTRACT_KEYWORDS: [&str; 22] = [
    "$schema",
    "$id",
    "$ref",
    "$defs",
    "title",
    "description",
    "type",
    "enum",
    "const",
    "required",
    "properties",
    "additionalProperties",
    "items",
    "pattern",
    "format",
    "minimum",
    "minLength",
    "maxLength",
    "allOf",
    "if",
    "then",
    "else",
];

/// Resolves `file#/json/pointer`, `#/json/pointer` or `file` against the vendored directory.
fn resolve(target: &str, root: &Value) -> (Value, Value) {
    let (file, pointer) = target.split_once('#').unwrap_or((target, ""));
    let base = if file.is_empty() {
        root.clone()
    } else {
        load(file)
    };
    let node = base
        .pointer(pointer)
        .unwrap_or_else(|| panic!("`$ref` {target} does not resolve"))
        .clone();
    (node, base)
}

/// A structural check of the contract's permitted keywords, not a general JSON Schema validator: no
/// crate that is one passes the repository's licence and MSRV policy. `format` is an annotation
/// here, as it is by default in draft 2020-12.
fn check(schema: &Value, root: &Value, instance: &Value, at: &str, errors: &mut Vec<String>) {
    let map = match schema {
        Value::Bool(true) => return,
        Value::Bool(false) => {
            errors.push(format!("{at}: not allowed"));
            return;
        }
        Value::Object(map) => map,
        _ => panic!("{at}: schema is neither an object nor a boolean"),
    };
    for (keyword, rule) in map {
        match keyword.as_str() {
            "$schema" | "$id" | "title" | "description" | "format" | "$defs" => {}
            "$ref" => {
                let (node, base) = resolve(rule.as_str().unwrap(), root);
                check(&node, &base, instance, at, errors);
            }
            "type" => {
                let names: Vec<&str> = match rule {
                    Value::String(n) => vec![n.as_str()],
                    other => other
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|n| n.as_str().unwrap())
                        .collect(),
                };
                if !names.iter().any(|n| is_type(instance, n)) {
                    errors.push(format!("{at}: expected {names:?}, got {instance}"));
                }
            }
            "required" => {
                for key in rule.as_array().unwrap() {
                    let key = key.as_str().unwrap();
                    if instance.is_object() && instance.get(key).is_none() {
                        errors.push(format!("{at}: missing required `{key}`"));
                    }
                }
            }
            "properties" => {
                for (key, sub) in rule.as_object().unwrap() {
                    if let Some(value) = instance.get(key) {
                        check(sub, root, value, &format!("{at}/{key}"), errors);
                    }
                }
            }
            "additionalProperties" => {
                let declared = map.get("properties").and_then(Value::as_object);
                for (key, value) in instance.as_object().into_iter().flatten() {
                    if declared.is_none_or(|d| !d.contains_key(key)) {
                        check(rule, root, value, &format!("{at}/{key}"), errors);
                    }
                }
            }
            "items" => {
                for (i, item) in instance.as_array().into_iter().flatten().enumerate() {
                    check(rule, root, item, &format!("{at}/{i}"), errors);
                }
            }
            "enum" => {
                if !rule.as_array().unwrap().contains(instance) {
                    errors.push(format!("{at}: {instance} is not one of {rule}"));
                }
            }
            "const" => {
                if rule != instance {
                    errors.push(format!("{at}: expected {rule}, got {instance}"));
                }
            }
            "minLength" | "maxLength" => {
                if let Some(text) = instance.as_str() {
                    let len = text.chars().count() as u64;
                    let bound = rule.as_u64().unwrap();
                    let fails = if keyword == "minLength" {
                        len < bound
                    } else {
                        len > bound
                    };
                    if fails {
                        errors.push(format!("{at}: length {len} fails {keyword} {bound}"));
                    }
                }
            }
            "minimum" => {
                if let Some(n) = instance.as_f64() {
                    if n < rule.as_f64().unwrap() {
                        errors.push(format!("{at}: {n} is below {rule}"));
                    }
                }
            }
            "pattern" => {
                let re = regex::Regex::new(rule.as_str().unwrap()).unwrap();
                if let Some(text) = instance.as_str() {
                    if !re.is_match(text) {
                        errors.push(format!("{at}: `{text}` does not match {rule}"));
                    }
                }
            }
            "allOf" => {
                for sub in rule.as_array().unwrap() {
                    check(sub, root, instance, at, errors);
                }
            }
            "if" => {
                let mut condition = Vec::new();
                check(rule, root, instance, at, &mut condition);
                let branch = if condition.is_empty() { "then" } else { "else" };
                if let Some(sub) = map.get(branch) {
                    check(sub, root, instance, at, errors);
                }
            }
            "then" | "else" => {
                assert!(map.contains_key("if"), "{at}: `{keyword}` without `if`");
            }
            other => panic!("{at}: keyword `{other}` is not supported by this structural check"),
        }
    }
}

fn is_type(value: &Value, name: &str) -> bool {
    match name {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        "number" => value.is_number(),
        "integer" => value.as_f64().is_some_and(|n| n.fract() == 0.0),
        other => panic!("type `{other}` is not supported by this structural check"),
    }
}

/// Every keyword in a schema position, walked through the subschema-bearing keywords. Each `pattern`
/// must compile and each `$ref` must resolve.
fn keywords_of(schema: &Value, root: &Value, found: &mut std::collections::BTreeSet<String>) {
    let Some(map) = schema.as_object() else {
        return;
    };
    for (keyword, rule) in map {
        found.insert(keyword.clone());
        match keyword.as_str() {
            "properties" | "$defs" => rule
                .as_object()
                .unwrap()
                .values()
                .for_each(|s| keywords_of(s, root, found)),
            "allOf" => rule
                .as_array()
                .unwrap()
                .iter()
                .for_each(|s| keywords_of(s, root, found)),
            "items" | "additionalProperties" | "if" | "then" | "else" => {
                keywords_of(rule, root, found)
            }
            "pattern" => {
                regex::Regex::new(rule.as_str().unwrap()).unwrap();
            }
            "$ref" => {
                resolve(rule.as_str().unwrap(), root);
            }
            _ => {}
        }
    }
}

fn validate(schema_file: &str, instance: &Value) -> Result<(), String> {
    let root = load(schema_file);
    let mut errors = Vec::new();
    check(&root, &root, instance, "", &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

fn run(dir: &Path, args: &[&str]) -> (Value, i32) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    let json = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout of {args:?} is not JSON ({e}): {}",
            String::from_utf8_lossy(&out.stdout)
        )
    });
    (json, out.status.code().unwrap())
}

fn fixture_corpus() -> common::TempDir {
    let dir = common::TempDir::new("urzua-contract");
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, field.quality: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "---\nStatus: Accepted\n---\n# 0001 — X\n",
    )
    .unwrap();
    // No Status: a blocking finding, so the report's findings array is not empty.
    std::fs::write(
        dir.join("docs/adr/0002-y.md"),
        "---\nTitle: y\n---\n# 0002 — Y\n",
    )
    .unwrap();
    common::git(&dir, &["init", "-q"]);
    common::git(&dir, &["add", "-A"]);
    dir
}

#[test]
fn the_vendored_copy_matches_its_own_checksums() {
    let sums = std::fs::read_to_string(schema_dir().join("SHA256SUMS")).unwrap();
    let mut listed = Vec::new();
    for line in sums.lines().filter(|l| !l.trim().is_empty()) {
        let (hash, name) = line.split_once("  ").expect("sha256sum line");
        let actual = urzua_io::sha256_hex_of_file(&schema_dir().join(name))
            .unwrap_or_else(|| panic!("{name} is listed in SHA256SUMS but unreadable"));
        assert_eq!(actual, hash, "{name} differs from its checksum");
        listed.push(name.to_string());
    }
    let mut on_disk: Vec<String> = std::fs::read_dir(schema_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".schema.json"))
        .collect();
    on_disk.sort();
    listed.sort();
    assert_eq!(on_disk, listed, "every vendored schema is checksummed");
    let version = std::fs::read_to_string(contract_dir().join("VERSION")).unwrap();
    assert_eq!(
        version.split_whitespace().count(),
        2,
        "VERSION is `<tag> <commit>`"
    );
}

#[test]
fn every_vendored_schema_uses_only_the_contract_keywords_and_every_pattern_compiles() {
    let mut found = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir(schema_dir()).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        if name.ends_with(".schema.json") {
            let root = load(&name);
            keywords_of(&root, &root, &mut found);
        }
    }
    let outside: Vec<&String> = found
        .iter()
        .filter(|k| !CONTRACT_KEYWORDS.contains(&k.as_str()))
        .collect();
    assert!(
        outside.is_empty(),
        "keywords outside the contract's list: {outside:?}"
    );
}

#[test]
fn the_check_answers_the_keywords_the_fixture_schemas_do_not_exercise() {
    let root = serde_json::json!({});
    let fails = |schema: Value, instance: Value| {
        let mut errors = Vec::new();
        check(&schema, &root, &instance, "", &mut errors);
        !errors.is_empty()
    };
    assert!(fails(
        serde_json::json!({"maxLength": 2}),
        serde_json::json!("abc")
    ));
    assert!(!fails(
        serde_json::json!({"maxLength": 3}),
        serde_json::json!("abc")
    ));
    assert!(fails(
        serde_json::json!({"pattern": "^[0-9]+$"}),
        serde_json::json!("1a")
    ));
    assert!(fails(
        serde_json::json!({"properties": {"a": {}}, "additionalProperties": false}),
        serde_json::json!({"a": 1, "b": 2})
    ));
    assert!(!fails(
        serde_json::json!({"properties": {"a": {}}, "additionalProperties": false}),
        serde_json::json!({"a": 1})
    ));
    assert!(!fails(
        serde_json::json!({"description": "x", "format": "date-time"}),
        serde_json::json!("not a date")
    ));
}

#[test]
fn check_reports_examined_files_equal_to_files_examined() {
    let dir = fixture_corpus();
    let (report, _) = run(&dir, &["check", "docs/"]);
    assert_eq!(report["files_examined"], 2);
    assert_eq!(report["examined"]["files"], report["files_examined"]);
}

#[test]
fn check_reports_validate_against_the_gate_report_schema() {
    let dir = fixture_corpus();
    let (report, code) = run(&dir, &["check", "docs/"]);
    assert_eq!(code, 1, "the fixture has one blocking finding");
    assert!(!report["findings"].as_array().unwrap().is_empty());
    validate("gate-report.schema.json", &report)
        .unwrap_or_else(|e| panic!("check report fails the gate profile: {e}"));

    let (not_run, code) = run(&dir, &["check", "--rules", "nope.nope"]);
    assert_eq!(code, 2);
    assert_eq!(not_run["status"], "not-run");
    assert!(
        not_run.get("examined").is_none(),
        "not-run invents no count"
    );
    validate("gate-report.schema.json", &not_run)
        .unwrap_or_else(|e| panic!("not-run report fails the gate profile: {e}"));
}

#[test]
fn a_report_with_placeholders_validates_against_the_base_schema() {
    let dir = fixture_corpus();
    let (report, _) = run(&dir, &["explain", "--types"]);
    validate("report.schema.json", &report)
        .unwrap_or_else(|e| panic!("explain report fails the base schema: {e}"));
}

#[test]
fn the_validator_rejects_a_gate_report_without_examined() {
    let dir = fixture_corpus();
    let (mut report, _) = run(&dir, &["check", "docs/"]);
    report.as_object_mut().unwrap().remove("examined");
    assert!(validate("gate-report.schema.json", &report).is_err());
}

#[test]
fn the_check_rejects_what_the_schemas_forbid() {
    let dir = fixture_corpus();
    let (report, _) = run(&dir, &["check", "docs/"]);
    let broken = |edit: &dyn Fn(&mut Value)| {
        let mut r = report.clone();
        edit(&mut r);
        validate("gate-report.schema.json", &r).is_err()
    };
    assert!(broken(
        &|r| r["examined"] = serde_json::json!({"files": "2"})
    ));
    assert!(broken(
        &|r| r["next_action_inputs"] = serde_json::json!(["<x>"])
    ));
    assert!(broken(
        &|r| r["findings"][0]["severity"] = serde_json::json!("warn")
    ));
    assert!(broken(&|r| r["findings"][0]["line"] = serde_json::json!(0)));
    let (mut not_run, _) = run(&dir, &["check", "--rules", "nope.nope"]);
    not_run["blocking"] = serde_json::json!(true);
    assert!(validate("gate-report.schema.json", &not_run).is_err());
}

#[test]
fn a_not_run_report_that_started_counts_zero_and_one_that_could_not_start_counts_nothing() {
    let dir = fixture_corpus();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/lib.rs"), "").unwrap();
    let (started, _) = run(&dir, &["check", "src/"]);
    assert_eq!(started["status"], "not-run");
    assert!(started.get("error").is_none());
    assert_eq!(started["files_examined"], 0);
    assert_eq!(started["examined"], serde_json::json!({"files": 0}));
    validate("gate-report.schema.json", &started)
        .unwrap_or_else(|e| panic!("started not-run report fails the gate profile: {e}"));

    let (failed, _) = run(&dir, &["check", "--rules", "nope.nope"]);
    assert!(failed.get("error").is_some());
    assert!(failed.get("files_examined").is_none());
    assert!(failed.get("examined").is_none());
}
