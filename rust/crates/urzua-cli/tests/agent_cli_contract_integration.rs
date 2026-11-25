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
    let mut walked = Vec::new();
    for entry in std::fs::read_dir(schema_dir()).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        if name.ends_with(".schema.json") {
            let root = load(&name);
            keywords_of(&root, &root, &mut found);
            walked.push(name);
        }
    }
    walked.sort();
    assert_eq!(
        walked,
        [
            "describe",
            "finding",
            "gate-report",
            "log-line",
            "receipt",
            "report",
            "run-report"
        ]
        .map(|n| format!("{n}.schema.json")),
        "the walk covers each of the contract's seven schemas"
    );
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

fn no_repository() -> common::TempDir {
    common::TempDir::new("urzua-contract-bare")
}

fn run_with_env(dir: &Path, args: &[&str], env: (&str, &str)) -> (Value, i32) {
    let out = common::urzua()
        .args(args)
        .env(env.0, env.1)
        .current_dir(dir)
        .output()
        .unwrap();
    let json = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout of {args:?} is not JSON ({e}): {out:?}"));
    (json, out.status.code().unwrap())
}

fn assert_not_run_of_kind(report: &Value, code: i32, kind: &str, what: &str) {
    assert_eq!(code, 2, "{what}: {report}");
    assert_eq!(report["status"], "not-run", "{what}: {report}");
    assert_eq!(
        report["error_kinds"],
        serde_json::json!([kind]),
        "{what}: {report}"
    );
    assert!(report.get("error_kind").is_none(), "{what}: {report}");
    assert!(report["error"].is_string(), "{what}: {report}");
    validate("report.schema.json", report)
        .unwrap_or_else(|e| panic!("{what}: fails the report schema: {e}"));
}

#[test]
fn every_not_run_report_carries_the_error_kind_of_its_cause() {
    let dir = fixture_corpus();

    let (usage, code) = run(&dir, &["check", "--no-such-flag"]);
    assert_not_run_of_kind(&usage, code, "usage", "a wrong argument");

    std::fs::write(dir.join(".urzua/config.yaml"), "rules: [unterminated\n").unwrap();
    let (config, code) = run(&dir, &["check"]);
    assert_not_run_of_kind(&config, code, "config", "an unreadable configuration");

    let bare = no_repository();
    let (environment, code) = run(&bare, &["check"]);
    assert_not_run_of_kind(&environment, code, "environment", "a missing repository");

    let (internal, code) = run_with_env(&dir, &["check"], ("URZUA_TEST_PANIC", "1"));
    assert_not_run_of_kind(&internal, code, "internal", "a forced internal failure");
    assert!(internal.get("panic").is_none(), "no panic key: {internal}");
}

#[test]
fn a_usage_error_names_its_argument_in_one_sentence_without_the_parsers_usage_block() {
    let dir = fixture_corpus();
    for args in [
        &["check", "--no-such-flag"][..],
        &["explain"][..],
        &["check", "--fail-on", "sideways"][..],
        &["frobnicate"][..],
    ] {
        let (report, code) = run(&dir, args);
        assert_not_run_of_kind(&report, code, "usage", &format!("{args:?}"));
        let error = report["error"].as_str().unwrap();
        assert!(!error.contains("Usage:"), "{args:?}: {error:?}");
        assert!(!error.contains('\n'), "{args:?}: {error:?}");
        assert!(!error.trim().is_empty(), "{args:?}");
    }
    let (report, _) = run(&dir, &["check", "--no-such-flag"]);
    assert!(
        report["error"].as_str().unwrap().contains("--no-such-flag"),
        "names the argument: {report}"
    );
}

fn describe() -> Value {
    let out = common::urzua().arg("describe").output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stderr.is_empty(), "describe writes nothing to stderr");
    serde_json::from_slice(&out.stdout).unwrap()
}

/// `describe` as a 1.1 tool would print it: the same object with the version
/// raised, so the schema's 1.1 requirements (`args`, `env`, `config`, `log`,
/// `stdin` and `protocol`) are checked although urzua still declares 1.0.
fn as_one_one(mut d: Value) -> Value {
    d["contract_version"] = serde_json::json!("1.1");
    d
}

/// What the schema cannot state and the contract's runner checks: names are
/// unique within a command together with the top-level `args`, and each name in
/// `conflicts_with`, `requires`, `requires_any` and `required_unless` is one of
/// them, and an argument with `required_unless` is not `required`.
fn argument_names_resolve(d: &Value) -> Result<(), String> {
    let names = |args: &Value| -> Vec<String> {
        args.as_array()
            .into_iter()
            .flatten()
            .map(|a| a["name"].as_str().unwrap().to_string())
            .collect()
    };
    let top = names(&d["args"]);
    for command in d["commands"].as_array().unwrap() {
        let mut all = top.clone();
        all.extend(names(&command["args"]));
        let mut sorted = all.clone();
        sorted.sort();
        sorted.dedup();
        if sorted.len() != all.len() {
            return Err(format!(
                "{}: argument names repeat: {all:?}",
                command["name"]
            ));
        }
        for arg in command["args"].as_array().into_iter().flatten() {
            if !arg["required_unless"].is_null() && arg["required"] == true {
                return Err(format!(
                    "{}: {} is required and has required_unless",
                    command["name"], arg["name"]
                ));
            }
            for key in [
                "conflicts_with",
                "requires",
                "requires_any",
                "required_unless",
            ] {
                for other in arg[key].as_array().into_iter().flatten() {
                    if !all.iter().any(|n| n == other.as_str().unwrap()) {
                        return Err(format!(
                            "{}: {} names {other} in {key}",
                            command["name"], arg["name"]
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn describe_validates_against_the_describe_schema() {
    let d = describe();
    assert_eq!(d["contract_version"], "1.0", "1.1 is declared by MILE-370");
    validate("describe.schema.json", &d).unwrap_or_else(|e| panic!("describe fails: {e}"));
    validate("describe.schema.json", &as_one_one(d.clone()))
        .unwrap_or_else(|e| panic!("describe fails the 1.1 requirements: {e}"));
    argument_names_resolve(&d).unwrap();
    for command in d["commands"].as_array().unwrap() {
        if command["profile"] == "exempt" {
            assert_eq!(command["safe"], false, "{}", command["name"]);
        }
    }
}

#[test]
fn the_describe_check_rejects_what_the_contract_forbids() {
    let d = as_one_one(describe());
    let broken = |edit: &dyn Fn(&mut Value)| {
        let mut r = d.clone();
        edit(&mut r);
        validate("describe.schema.json", &r).is_err() || argument_names_resolve(&r).is_err()
    };
    let at = |name: &str| {
        d["commands"]
            .as_array()
            .unwrap()
            .iter()
            .position(|c| c["name"] == name)
            .unwrap()
    };
    let (mcp, check, board) = (at("mcp"), at("check"), at("board"));
    assert!(broken(&|r| {
        r["commands"][mcp]
            .as_object_mut()
            .unwrap()
            .remove("protocol");
    }));
    assert!(broken(&|r| {
        r["commands"][check]
            .as_object_mut()
            .unwrap()
            .remove("stdin");
    }));
    assert!(broken(&|r| {
        r.as_object_mut().unwrap().remove("env");
    }));
    assert!(broken(&|r| {
        r["commands"][check]["args"][0]
            .as_object_mut()
            .unwrap()
            .remove("type");
    }));
    assert!(broken(
        &|r| r["commands"][check]["args"][0]["kind"] = serde_json::json!("switch")
    ));
    assert!(broken(
        &|r| r["commands"][board]["args"][3]["requires"] = serde_json::json!(["--nope"])
    ));
    assert!(broken(
        &|r| r["commands"][check]["args"][0]["name"] = serde_json::json!("--config")
    ));
    let explain = at("explain");
    assert!(broken(&|r| r["commands"][explain]["args"][0]
        ["required_unless"] =
        serde_json::json!(["--nope"])));
    assert!(broken(&|r| r["commands"][explain]["args"][0]
        ["requires_any"] =
        serde_json::json!("--rule")));
    assert!(broken(
        &|r| r["commands"][explain]["args"][0]["required"] = serde_json::json!(true)
    ));
}

/// The words that give a described argument a value it accepts.
fn tokens(arg: &Value) -> Vec<String> {
    let value = arg["values"][0]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| {
            if arg["type"] == "integer" {
                "0".into()
            } else {
                "x".into()
            }
        });
    let name = arg["name"].as_str().unwrap().to_string();
    match arg["kind"].as_str().unwrap() {
        "positional" => vec![value],
        "flag" => vec![name],
        _ => vec![name, value],
    }
}

/// The command at `words`, with every required argument but `except`, then `extra`.
fn invocation(command: &Value, except: &[&str], extra: &[&Value]) -> Vec<String> {
    let mut argv: Vec<String> = command["name"]
        .as_str()
        .unwrap()
        .split(' ')
        .map(String::from)
        .collect();
    for arg in command["args"].as_array().unwrap() {
        let name = arg["name"].as_str().unwrap();
        if arg["required"] == true
            && !except.contains(&name)
            && !extra.iter().any(|e| e["name"] == name)
        {
            argv.extend(tokens(arg));
        }
    }
    for arg in extra {
        argv.extend(tokens(arg));
    }
    argv
}

/// stdout's report, or stderr's for `mcp`, which keeps stdout for protocol frames.
fn refused(dir: &Path, argv: &[String]) -> Result<(), String> {
    let out = common::urzua()
        .args(argv)
        .current_dir(dir)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    let text = if out.stdout.is_empty() {
        &out.stderr
    } else {
        &out.stdout
    };
    let report: Value = serde_json::from_slice(text).unwrap_or(Value::Null);
    if out.status.code() == Some(2) && report["status"] == "not-run" {
        Ok(())
    } else {
        Err(format!(
            "{argv:?} was not refused: exit {:?}, {}",
            out.status.code(),
            String::from_utf8_lossy(text)
        ))
    }
}

fn arg_of<'a>(command: &'a Value, name: &str) -> &'a Value {
    command["args"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == name)
        .unwrap()
}

/// Whether `text` names `name` as a whole word: `--types` does not name `--type`.
fn names_whole(text: &str, name: &str) -> bool {
    let word = format!(
        r"(?:^|[^a-z0-9_-]){}(?:$|[^a-z0-9_-])",
        regex::escape(&name.to_lowercase())
    );
    regex::Regex::new(&word).unwrap().is_match(text)
}

#[test]
fn a_longer_name_does_not_name_a_shorter_one() {
    assert!(!names_whole("--full goes with --types", "--type"));
    assert!(names_whole(
        "cannot be combined with paths or --base",
        "--base"
    ));
    assert!(names_whole("with paths or", "paths"));
    assert!(!names_whole("--stdin-path", "--stdin"));
}

/// A refusal whose error names both arguments (`paths` is written `PATHS`), so
/// it refuses the pair and not some other mistake in the probe.
fn refused_naming(dir: &Path, argv: &[String], names: &[&str]) -> Result<(), String> {
    refused_naming_some(dir, argv, names, false)
}

/// As `refused_naming`, but one of `names` is enough when `any` is set.
fn refused_naming_some(
    dir: &Path,
    argv: &[String],
    names: &[&str],
    any: bool,
) -> Result<(), String> {
    refused(dir, argv)?;
    let out = common::urzua()
        .args(argv)
        .current_dir(dir)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&[out.stdout, out.stderr].concat()).to_lowercase();
    if names.iter().all(|n| names_whole(&text, n))
        || (any && names.iter().any(|n| names_whole(&text, n)))
    {
        Ok(())
    } else {
        Err(format!("{argv:?} was refused for another reason: {text}"))
    }
}

/// `args`, plus what they require and the first name of each `requires_any`
/// (so that a probe reaches the relation it tests), less `except`.
fn with_companions<'a>(command: &'a Value, args: &[&'a Value], except: &str) -> Vec<&'a Value> {
    let mut all: Vec<&Value> = args.to_vec();
    let mut i = 0;
    while i < all.len() {
        let mut next: Vec<&str> = all[i]["requires"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|n| n.as_str().unwrap())
            .collect();
        if let Some(first) = all[i]["requires_any"].get(0) {
            next.push(first.as_str().unwrap());
        }
        for name in next {
            if name != except && !all.iter().any(|x| x["name"] == name) {
                all.push(arg_of(command, name));
            }
        }
        i += 1;
    }
    all
}

/// Every declared `conflicts_with` and `requires` is refused by the binary:
/// both arguments together, or the argument without what it requires, is a
/// `not-run`. A relation `describe` declares and the command accepts fails here.
#[test]
fn every_declared_relation_is_refused() {
    let dir = fixture_corpus();
    let d = describe();
    let mut accepted = Vec::new();
    for command in d["commands"].as_array().unwrap() {
        for arg in command["args"].as_array().unwrap() {
            for other in arg["conflicts_with"].as_array().into_iter().flatten() {
                let other = arg_of(command, other.as_str().unwrap());
                let argv = invocation(command, &[], &with_companions(command, &[arg, other], ""));
                let names = [
                    arg["name"].as_str().unwrap(),
                    other["name"].as_str().unwrap(),
                ];
                // A companion one of them requires may conflict first, and the
                // one-sentence error names only the pair the parser met.
                accepted.extend(refused_naming_some(&dir, &argv, &names, true).err());
            }
            if let Some(first) = arg["requires_any"].get(0) {
                let first = first.as_str().unwrap();
                let argv = invocation(command, &[], &with_companions(command, &[arg], first));
                let mut names = vec![arg["name"].as_str().unwrap()];
                names.extend(
                    arg["requires_any"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|n| n.as_str().unwrap()),
                );
                accepted.extend(refused_naming(&dir, &argv, &names).err());
            }
            if arg["required_unless"].is_array() {
                let argv = invocation(command, &[arg["name"].as_str().unwrap()], &[]);
                let names = [arg["name"].as_str().unwrap()];
                accepted.extend(refused_naming(&dir, &argv, &names).err());
            }
            for needed in arg["requires"].as_array().into_iter().flatten() {
                let needed = needed.as_str().unwrap();
                let argv = invocation(
                    command,
                    &[needed],
                    &with_companions(command, &[arg], needed),
                );
                // The error names the argument that is missing, not the one that required it.
                accepted.extend(refused_naming(&dir, &argv, &[needed]).err());
            }
        }
    }
    assert!(accepted.is_empty(), "{}", accepted.join("\n"));
}

/// `--as-of` is declared on exactly the commands that accept it: every other
/// command refuses it, and each declaring command runs with it.
#[test]
fn as_of_is_declared_where_it_is_accepted() {
    let dir = fixture_corpus();
    let d = describe();
    let as_of = serde_json::json!({"name": "--as-of", "kind": "option", "type": "string", "values": ["2026-01-01"]});
    for command in d["commands"].as_array().unwrap() {
        let declared = command["args"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["name"] == "--as-of");
        let argv = with_example(command, &[&as_of]);
        if declared {
            let out = common::urzua()
                .args(&argv)
                .current_dir(&dir)
                .stdin(std::process::Stdio::null())
                .output()
                .unwrap();
            let text = String::from_utf8_lossy(&out.stdout);
            assert!(
                !text.contains("--as-of is not accepted"),
                "{argv:?}: {text}"
            );
        } else {
            let out = common::urzua()
                .args(&argv)
                .current_dir(&dir)
                .stdin(std::process::Stdio::null())
                .output()
                .unwrap();
            let text = [out.stdout, out.stderr].concat();
            assert!(
                String::from_utf8_lossy(&text).contains("--as-of is not accepted"),
                "{argv:?} accepted --as-of"
            );
        }
    }
}

/// The command's `example`, as arguments.
fn example(command: &Value) -> Vec<String> {
    command["example"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

/// The command run as its `example` (or with its required arguments when it
/// has none), then `extra`.
fn with_example(command: &Value, extra: &[&Value]) -> Vec<String> {
    let example = example(command);
    if example.is_empty() {
        return invocation(command, &[], extra);
    }
    let mut argv: Vec<String> = command["name"]
        .as_str()
        .unwrap()
        .split(' ')
        .map(String::from)
        .collect();
    argv.extend(example);
    for arg in extra {
        argv.extend(tokens(arg));
    }
    argv
}

fn stdout_of(dir: &Path, argv: &[String]) -> (Option<i32>, Vec<u8>) {
    let out = common::urzua()
        .args(argv)
        .current_dir(dir)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    (out.status.code(), out.stdout)
}

/// A safe command declares `--config` exactly when it reads the file it names:
/// pointing it at a missing file changes its answer. `rules`, which reads no
/// configuration and does not declare it, answers the same. A command refused
/// before it reads anything (`import`, `migrate schema` without `--field`)
/// says nothing either way.
#[test]
fn config_is_declared_where_it_is_read() {
    let dir = fixture_corpus();
    let d = describe();
    let missing = serde_json::json!({"name": "--config", "kind": "option", "type": "path", "values": ["no/such/config.yaml"]});
    for command in d["commands"].as_array().unwrap() {
        if command["safe"] != true || command["profile"] == "exempt" {
            continue;
        }
        let declared = command["args"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["name"] == "--config");
        let plain = with_example(command, &[]);
        if refused(&dir, &plain).is_ok() {
            continue;
        }
        let argv = with_example(command, &[&missing]);
        let changed = stdout_of(&dir, &plain) != stdout_of(&dir, &argv);
        assert_eq!(changed, declared, "{argv:?}");
    }
}

/// Every argument declared `required` is refused when left out, by an error
/// that names it.
#[test]
fn every_required_argument_is_refused_when_missing() {
    let dir = fixture_corpus();
    let d = describe();
    let mut accepted = Vec::new();
    for command in d["commands"].as_array().unwrap() {
        for arg in command["args"].as_array().unwrap() {
            if arg["required"] != true {
                continue;
            }
            let name = arg["name"].as_str().unwrap();
            // A later positional given alone would fill this one's place.
            let omitted: Vec<&str> = if arg["kind"] == "positional" {
                command["args"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .skip_while(|a| a["name"] != name)
                    .filter(|a| a["kind"] == "positional")
                    .map(|a| a["name"].as_str().unwrap())
                    .collect()
            } else {
                vec![name]
            };
            let argv = invocation(command, &omitted, &[]);
            let shown = if arg["kind"] == "positional" {
                name.to_uppercase()
            } else {
                name.to_string()
            };
            accepted.extend(refused_naming(&dir, &argv, &[shown.as_str(), shown.as_str()]).err());
        }
    }
    assert!(accepted.is_empty(), "{}", accepted.join("\n"));
}
