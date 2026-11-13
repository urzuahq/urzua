//! MILE-7: a spec whose Status says the work shipped must carry Embodiment and
//! Realized-by. Runs this repository's own `.urzua/config.yaml`, so the shipped
//! declaration is what is exercised.

mod common;

use std::path::Path;
use std::process::Command;

fn run_with_own_config(name: &str, spec: &str) -> serde_json::Value {
    let dir = common::TempDir::new(&format!("urzua-shipped-{name}"));
    std::fs::create_dir_all(dir.join("docs/specs")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        common::own_config_for_fixture(),
    )
    .unwrap();
    std::fs::write(dir.join("docs/specs/SPEC-1-first.md"), spec).unwrap();
    assert!(Command::new("git")
        .args(["add", "-A"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    let out = common::urzua()
        .args(["check", "docs/"])
        .current_dir(&dir)
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {out:?}"))
}

fn spec(status: &str, extra: &str) -> String {
    format!(
        "---\nStatus: {status}\nDate: 2026-11-04\nVersion: '0.1'\nAuthor: Ana\nSubject: 'x'\n{extra}---\n# 1 - First\n"
    )
}

fn rules(v: &serde_json::Value, rule: &str) -> usize {
    v["findings"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|f| f["rule"] == rule)
        .count()
}

#[test]
fn a_shipped_spec_without_embodiment_and_realized_by_is_reported_observed_failing() {
    let v = run_with_own_config("missing", &spec("Shipped", ""));
    assert_eq!(rules(&v, "status.undeclared-value"), 0, "{v}");
    assert_eq!(rules(&v, "status.required-field-missing"), 2, "{v}");
}

#[test]
fn a_shipped_spec_carrying_both_fields_is_clean_of_the_requirement() {
    let v = run_with_own_config(
        "complete",
        &spec(
            "Shipped",
            "Embodiment: Not started\nRealized-by: 'code:rust/crates'\n",
        ),
    );
    assert_eq!(rules(&v, "status.undeclared-value"), 0, "{v}");
    assert_eq!(rules(&v, "status.required-field-missing"), 0, "{v}");
}

#[test]
fn an_accepted_or_draft_spec_is_not_asked_for_them() {
    for s in ["Accepted", "Draft"] {
        let v = run_with_own_config(s, &spec(s, ""));
        assert_eq!(rules(&v, "status.required-field-missing"), 0, "{s}: {v}");
    }
}
