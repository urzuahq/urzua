//! `urzua explain --rule <rule-id>` (`ADR-96` amendment of 2026-11-07,
//! `SPEC-13`): what one rule is, as data -- its scope, the level this
//! repository sets, its severity floor and waivable flag, its options with
//! their current values, its `ADR-73` action variants, its population unit and
//! its spec. It reads the same rule table `urzua rules` prints.

mod common;

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
}

fn repo(name: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-explain-rule-{name}"));
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    for d in ["docs/adr", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        "---\nStatus: Draft\n---\n# 1 \u{2014} A\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

const CONFIG: &str = "schema_version: 2\nrules:\n  status.delivery-agreement: warn\n  pointer.target-status: {level: error, not_in: [Rejected, Withdrawn]}\nrecord_types:\n  adr:\n    dir: docs/adr\n    required_fields: []\n";

fn run(dir: &Path, args: &[&str]) -> (std::process::Output, serde_json::Value) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    let parsed = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not JSON ({e}): {}",
            String::from_utf8_lossy(&out.stdout)
        )
    });
    (out, parsed)
}

fn variant<'a>(rule: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    rule["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["variant"] == name)
        .unwrap_or_else(|| panic!("no variant {name}: {rule}"))
}

#[test]
fn a_rule_prints_its_identity_scope_level_floor_and_waivable_flag() {
    let dir = repo("identity", CONFIG);
    let (out, report) = run(&dir, &["explain", "--rule", "status.delivery-agreement"]);
    assert_eq!(out.status.code(), Some(0), "{report}");
    assert_eq!(report["status"], "ok", "{report}");
    let rule = &report["rule"];
    assert_eq!(rule["id"], "status.delivery-agreement");
    assert_eq!(rule["scope"], "records");
    assert_eq!(rule["level"], "warn");
    assert_eq!(rule["severity_floor"], "error", "{report}");
    assert_eq!(rule["waivable"], false, "{report}");
    let floor = variant(rule, "done-with-unresolved-items");
    assert_eq!(floor["severity_floor"], "error");
    assert_eq!(floor["waivable"], false);
    // A variant with no floor is waivable, and says nothing about a floor.
    let plain = variant(rule, "delivery-not-done");
    assert!(plain.get("severity_floor").is_none(), "{plain}");
    assert!(plain.get("waivable").is_none(), "{plain}");
}

#[test]
fn a_rule_the_repository_does_not_configure_is_off() {
    let dir = repo("off", CONFIG);
    let (_, report) = run(&dir, &["explain", "--rule", "header.required-fields"]);
    assert_eq!(report["rule"]["level"], "off", "{report}");
}

#[test]
fn options_are_listed_with_their_current_values() {
    let dir = repo("options", CONFIG);
    let (_, report) = run(&dir, &["explain", "--rule", "pointer.target-status"]);
    let options = &report["rule"]["options"];
    assert_eq!(
        options["not_in"],
        serde_json::json!(["Rejected", "Withdrawn"])
    );
    // A declared option nobody set is listed as null, not left out.
    assert!(
        options.as_object().unwrap().contains_key("fields"),
        "{report}"
    );
    assert!(options["fields"].is_null(), "{report}");
}

#[test]
fn a_rule_without_options_has_an_empty_options_object() {
    let dir = repo("no-options", CONFIG);
    let (_, report) = run(&dir, &["explain", "--rule", "header.required-fields"]);
    assert_eq!(report["rule"]["options"], serde_json::json!({}), "{report}");
}

#[test]
fn action_variants_carry_their_adr_73_kind_or_exemption() {
    let dir = repo("actions", CONFIG);
    let (_, report) = run(&dir, &["explain", "--rule", "header.required-fields"]);
    let rule = &report["rule"];
    assert_eq!(variant(rule, "field-missing")["kind"], "edit-field");
    assert!(variant(rule, "header-region-missing")["exempt"].is_string());
}

#[test]
fn the_population_unit_and_spec_are_named() {
    let dir = repo("unit", CONFIG);
    let (_, report) = run(&dir, &["explain", "--rule", "header.required-fields"]);
    assert_eq!(report["rule"]["population_unit"], "field", "{report}");
    assert_eq!(report["rule"]["spec"], "SPEC-2", "{report}");
}

#[test]
fn the_rule_entry_is_the_one_urzua_rules_prints() {
    let dir = repo("same-data", CONFIG);
    let (_, rules) = run(&dir, &["rules"]);
    for expected in rules["rules"].as_array().unwrap() {
        let id = expected["id"].as_str().unwrap();
        let (out, report) = run(&dir, &["explain", "--rule", id]);
        assert_eq!(out.status.code(), Some(0), "{id}: {report}");
        let rule = &report["rule"];
        for key in [
            "id",
            "description",
            "scope",
            "judges",
            "actions",
            "population_unit",
            "spec",
        ] {
            assert_eq!(rule[key], expected[key], "{id}.{key}");
        }
        let names: Vec<&str> = rule["options"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let mut declared: Vec<&str> = expected["options"]
            .as_array()
            .map(|o| o.iter().map(|v| v.as_str().unwrap()).collect())
            .unwrap_or_default();
        declared.sort_unstable();
        let mut names = names;
        names.sort_unstable();
        assert_eq!(names, declared, "{id} options");
        assert!(rule["population_unit"].is_string(), "{id}: {rule}");
    }
}

#[test]
fn an_unknown_rule_id_is_an_error_naming_the_nearest_valid_id() {
    let dir = repo("unknown", CONFIG);
    let (out, report) = run(&dir, &["explain", "--rule", "status.delivery-agrement"]);
    assert_eq!(out.status.code(), Some(2), "{report}");
    let text = report["error"].as_str().unwrap();
    assert!(text.contains("status.delivery-agrement"), "{text}");
    assert!(
        text.contains("did you mean 'status.delivery-agreement'?"),
        "{text}"
    );
}

#[test]
fn an_unknown_rule_id_far_from_every_rule_names_no_guess() {
    let dir = repo("far", CONFIG);
    let (out, report) = run(&dir, &["explain", "--rule", "zzzzzzzzzzzzzzzzzzzz"]);
    assert_eq!(out.status.code(), Some(2), "{report}");
    let text = report["error"].as_str().unwrap();
    assert!(!text.contains("did you mean"), "{text}");
    assert!(text.contains("urzua rules"), "{text}");
}

#[test]
fn a_positional_argument_is_always_a_path_even_when_it_spells_a_rule_id() {
    // A file that really has the spelling of a rule id is explained as a path.
    let dir = repo("positional", CONFIG);
    std::fs::write(dir.join("status.delivery-agreement"), "x").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "file"]);
    let (out, report) = run(&dir, &["explain", "status.delivery-agreement"]);
    assert_eq!(out.status.code(), Some(0), "{report}");
    assert_eq!(report["path"], "status.delivery-agreement", "{report}");
    assert!(report.get("rule").is_none(), "{report}");
    assert!(report["governing_records"].is_array(), "{report}");
}

#[test]
fn a_path_that_names_no_file_and_no_cited_evidence_is_an_error() {
    let dir = repo("nosuchfile", CONFIG);
    let (out, report) = run(&dir, &["explain", "nosuchfile.rs"]);
    assert_eq!(out.status.code(), Some(2), "{report}");
    assert_eq!(report["status"], "not-run", "{report}");
    assert!(
        report.to_string().contains("nosuchfile.rs"),
        "the error names the path: {report}"
    );
}

#[test]
fn a_rule_id_given_as_a_path_is_an_error_that_points_at_rule() {
    let dir = repo("ruleaspath", CONFIG);
    let (out, report) = run(&dir, &["explain", "status.delivery-agreement"]);
    assert_eq!(out.status.code(), Some(2), "{report}");
    assert!(
        report.to_string().contains("--rule"),
        "the error says to use --rule: {report}"
    );
}

#[test]
fn a_deleted_file_that_a_record_still_cites_is_explained() {
    let dir = repo("deleted-cited", CONFIG);
    std::fs::write(
        dir.join("docs/adr/ADR-2-b.md"),
        "---\nStatus: Accepted\nRealized-by: code:src/gone.rs\n---\n# 2 \u{2014} B\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "cites"]);
    let (out, report) = run(&dir, &["explain", "src/gone.rs"]);
    assert_eq!(out.status.code(), Some(0), "{report}");
    assert_eq!(
        report["governing_records"].as_array().map(Vec::len),
        Some(1),
        "{report}"
    );
}

#[test]
fn a_path_and_a_rule_cannot_be_given_together() {
    let dir = repo("both", CONFIG);
    let out = common::urzua()
        .args([
            "explain",
            "docs/adr/ADR-1-a.md",
            "--rule",
            "status.delivery-agreement",
        ])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["status"], "not-run", "{report}");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("cannot be used with"),
        "{report}"
    );
}

#[test]
fn explain_with_neither_a_path_nor_a_rule_is_a_usage_error() {
    let dir = repo("neither", CONFIG);
    let out = common::urzua()
        .arg("explain")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let text = report["error"].as_str().unwrap();
    assert!(text.contains("urzua explain --rule <RULE_ID>"), "{text}");
}

/// The unit a rule declares is the unit it reports: every rule this
/// repository's own configuration runs reports the unit `explain --rule`
/// prints for it.
#[test]
fn the_declared_population_unit_is_the_one_the_rule_reports() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let (_, check) = run(&root, &["check"]);
    let executed = check["rules_executed"].as_array().unwrap();
    assert!(!executed.is_empty());
    let (_, rules) = run(&root, &["rules"]);
    let mut compared = 0;
    for e in executed {
        let Some(unit) = e["population"]["unit"].as_str() else {
            continue;
        };
        let id = e["rule"].as_str().unwrap();
        let declared = rules["rules"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == id)
            .unwrap();
        // A rule with a companion population reports a second unit under its
        // id; the declared unit is the first.
        if executed.iter().filter(|x| x["rule"] == id).count() > 1 {
            continue;
        }
        assert_eq!(declared["population_unit"], unit, "{id}");
        compared += 1;
    }
    assert!(compared > 20, "only {compared} rules compared");
}
