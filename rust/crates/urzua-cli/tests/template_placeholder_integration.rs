//! BUG-22: with `template_placeholders: true`, `field.quality` reads a field
//! value identical to its type's template text for that field as a
//! Placeholder. Off unless declared.

mod common;

use std::process::Command;

const BUG_TEMPLATE: &str = "# NNNN — Title\n\n> Status: Open | Fixed | WontFix\n> Found-in: how/where this was actually discovered\n> Regression-test: path/name of the test proving this is fixed\n\n## What was wrong\n\nx\n";

fn config(option: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n  field.quality:\n    level: error\n{option}record_types:\n  bug:\n    dir: docs/bugs\n    header_shape: yaml-frontmatter\n    required_fields: [Status, Found-in, Regression-test]\n    statuses:\n      values: [Open, Fixed]\n      pending: [\"not yet written\"]\n  note:\n    dir: docs/notes\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n"
    )
}

fn run(name: &str, config: &str, template: Option<&str>, bug: &str) -> (serde_json::Value, i32) {
    let dir = common::TempDir::new(&format!("urzua-tplph-{name}"));
    std::fs::create_dir_all(dir.join("docs/bugs")).unwrap();
    std::fs::create_dir_all(dir.join("docs/notes")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua/templates")).unwrap();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    if let Some(t) = template {
        std::fs::write(dir.join(".urzua/templates/bug.md"), t).unwrap();
    }
    std::fs::write(dir.join("docs/bugs/BUG-1-first.md"), bug).unwrap();
    std::fs::write(
        dir.join("docs/notes/NOTE-1-first.md"),
        "---\nStatus: Open | Fixed\n---\n# 1 - First\n",
    )
    .unwrap();
    assert!(Command::new("git")
        .args(["add", "-A"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    let out = common::urzua()
        .args(["check"])
        .current_dir(&dir)
        .output()
        .expect("urzua binary should run");
    let v =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {out:?}"));
    (v, out.status.code().unwrap_or(-1))
}

fn quality_fields(v: &serde_json::Value) -> Vec<String> {
    v["findings"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|f| f["rule"] == "field.quality")
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect()
}

fn bug(status: &str, found: &str, test: &str) -> String {
    format!(
        "---\nStatus: {status}\nFound-in: '{found}'\nRegression-test: '{test}'\n---\n# 1 - First\n"
    )
}

#[test]
fn a_field_equal_to_its_template_text_is_a_placeholder_observed_failing() {
    let (v, _) = run(
        "equal",
        &config("    template_placeholders: true\n"),
        Some(BUG_TEMPLATE),
        &bug(
            "Open",
            "how/where this was actually discovered",
            "path/name of the test proving this is fixed",
        ),
    );
    let msgs = quality_fields(&v);
    assert_eq!(msgs.len(), 2, "{v}");
    assert!(msgs
        .iter()
        .any(|m| m.contains("Found-in") && m.contains("Placeholder")));
    assert!(msgs.iter().any(|m| m.contains("Regression-test")));
}

#[test]
fn the_whole_enumerated_status_template_value_is_a_placeholder() {
    let (v, _) = run(
        "status",
        &config("    template_placeholders: true\n"),
        Some(BUG_TEMPLATE),
        &bug("Open | Fixed | WontFix", "a real discovery", "a::real_test"),
    );
    let msgs = quality_fields(&v);
    assert_eq!(msgs.len(), 1, "{v}");
    assert!(msgs[0].contains("'Status'"));
}

#[test]
fn an_edited_value_and_a_trimmed_copy_are_distinguished() {
    let (v, _) = run(
        "edited",
        &config("    template_placeholders: true\n"),
        Some(BUG_TEMPLATE),
        &bug(
            "Open",
            "how/where this was actually discovered, in review of PR 9",
            "a::real_test",
        ),
    );
    assert!(quality_fields(&v).is_empty(), "{v}");
    let (v, _) = run(
        "padded",
        &config("    template_placeholders: true\n"),
        Some(BUG_TEMPLATE),
        &bug(
            "Open",
            "  how/where this was actually discovered ",
            "a::real_test",
        ),
    );
    assert_eq!(quality_fields(&v).len(), 1, "{v}");
}

#[test]
fn the_compiled_in_token_list_does_not_apply_once_declared() {
    // `tbd` is a built-in token; with template placeholders declared it is a
    // value like any other, because the vocabulary is the template's.
    let (v, _) = run(
        "tokens",
        &config("    template_placeholders: true\n"),
        Some(BUG_TEMPLATE),
        &bug("Open", "tbd", "a::real_test"),
    );
    assert!(quality_fields(&v).is_empty(), "{v}");
}

#[test]
fn without_the_declaration_nothing_changes() {
    let (v, _) = run(
        "off",
        &config(""),
        Some(BUG_TEMPLATE),
        &bug(
            "Open",
            "how/where this was actually discovered",
            "path/name of the test proving this is fixed",
        ),
    );
    assert!(quality_fields(&v).is_empty(), "{v}");
    let (v, _) = run("off-token", &config(""), None, &bug("Open", "tbd", "x"));
    assert_eq!(quality_fields(&v).len(), 1, "{v}");
}

#[test]
fn a_type_with_no_template_is_judged_by_its_pending_list_only() {
    // The `note` type has no template and a value that looks like one.
    let (v, _) = run(
        "no-template",
        &config("    template_placeholders: true\n"),
        Some(BUG_TEMPLATE),
        &bug("Open", "a real discovery", "a::real_test"),
    );
    assert!(quality_fields(&v).is_empty(), "{v}");
}

#[test]
fn declaring_it_with_no_template_anywhere_is_disclosed_not_silently_inert() {
    let (v, _) = run(
        "inert",
        &config("    template_placeholders: true\n"),
        None,
        &bug("Open", "a real discovery", "a::real_test"),
    );
    assert!(
        v["notices"].to_string().contains("template_placeholders"),
        "{v}"
    );
}

#[test]
fn the_option_is_refused_on_a_rule_that_does_not_read_it() {
    let cfg = config("").replace(
        "rules:\n",
        "rules:\n  field.pending:\n    level: warn\n    template_placeholders: true\n",
    );
    let (v, code) = run(
        "wrong-rule",
        &cfg,
        Some(BUG_TEMPLATE),
        &bug("Open", "x", "y"),
    );
    assert_ne!(code, 0, "{v}");
}

#[test]
fn migrate_schema_report_judges_a_field_as_field_quality_does() {
    // `tbd` is Present once the template is the vocabulary (the rule agrees);
    // a value equal to the template's text is a Placeholder.
    let cfg = config("    template_placeholders: true\n");
    let dir = common::TempDir::new("urzua-tplph-migrate");
    std::fs::create_dir_all(dir.join("docs/bugs")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua/templates")).unwrap();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    std::fs::write(dir.join(".urzua/config.yaml"), cfg).unwrap();
    std::fs::write(dir.join(".urzua/templates/bug.md"), BUG_TEMPLATE).unwrap();
    std::fs::write(
        dir.join("docs/bugs/BUG-1-first.md"),
        bug("Open", "tbd", "path/name of the test proving this is fixed"),
    )
    .unwrap();
    assert!(Command::new("git")
        .args(["add", "-A"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    let report = |field: &str| -> serde_json::Value {
        let out = common::urzua()
            .args(["migrate", "schema", "--report", "--field", field])
            .current_dir(&dir)
            .output()
            .unwrap();
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {out:?}"))
    };
    assert!(report("Found-in")["would_fail"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        report("Regression-test")["would_fail"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
