//! `urzua check`'s report against the published agent CLI contract, vendored under
//! `fixtures/agent-cli-contract/` from a release tag (`MILE-358`, `MILE-359`, `ADR-106`).
//! The check reads the local copy only: a `$ref` is answered from the vendored directory, never from
//! the network. It is a structural check of the schemas' stated requirements, not a general JSON
//! Schema validator.

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

/// A structural check of the keywords the vendored schemas state, not a general JSON Schema
/// validator: no crate that is one passes the repository's licence and MSRV policy. It supports
/// exactly the keywords these five files use, and panics on any other, so a schema that grows a rule
/// this check cannot read fails the test instead of passing unchecked.
fn check(schema: &Value, root: &Value, instance: &Value, at: &str, errors: &mut Vec<String>) {
    let Some(map) = schema.as_object() else {
        panic!("{at}: schema is not an object");
    };
    for (keyword, rule) in map {
        match keyword.as_str() {
            "$schema" | "$id" | "title" | "$defs" => {}
            "$ref" => {
                let target = rule.as_str().unwrap();
                match target.strip_prefix("#/$defs/") {
                    Some(def) => check(&root["$defs"][def], root, instance, at, errors),
                    None => {
                        let file = load(target);
                        check(&file, &file, instance, at, errors);
                    }
                }
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
            "minLength" => {
                if let Some(text) = instance.as_str() {
                    if (text.chars().count() as u64) < rule.as_u64().unwrap() {
                        errors.push(format!("{at}: shorter than {rule}"));
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
            "additionalProperties" => {
                for (key, value) in instance.as_object().into_iter().flatten() {
                    check(rule, root, value, &format!("{at}/{key}"), errors);
                }
            }
            "pattern" => {
                assert_eq!(rule, "^<.+>$", "only the placeholder pattern is supported");
                if let Some(text) = instance.as_str() {
                    let inner = text.strip_prefix('<').and_then(|t| t.strip_suffix('>'));
                    if inner.is_none_or(str::is_empty) {
                        errors.push(format!("{at}: `{text}` is not a <placeholder>"));
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
        "integer" => value.is_i64() || value.is_u64(),
        other => panic!("type `{other}` is not supported by this structural check"),
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
