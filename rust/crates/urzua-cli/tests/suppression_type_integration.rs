//! BUG-74: which record type suppresses findings is a declaration
//! (`role: suppression`), not the literal type name `waiver`.

use std::path::Path;
use std::process::Command;

fn config(types: &str) -> String {
    format!(
        "schema_version: 2
rules:
  header.required-fields: error
record_types:
  adr:
    dir: docs/adr
    required_fields: [Status, Author]
    header_shape: yaml-frontmatter
{types}"
    )
}

fn type_block(name: &str, dir: &str, role: bool) -> String {
    let role = if role { "    role: suppression\n" } else { "" };
    format!(
        "  {name}:\n    dir: {dir}\n{role}    required_fields: [Rule, Scope]\n    header_shape: yaml-frontmatter\n"
    )
}

/// ADR-1 lacks Author, a live `header.required-fields` finding. Each
/// `(dir, file)` holds a record covering it.
fn corpus(name: &str, config: &str, records: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-supp-type-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        "---\nStatus: Accepted\n---\n# 1 - A\n",
    )
    .unwrap();
    for (d, file) in records {
        std::fs::create_dir_all(dir.join(d)).unwrap();
        std::fs::write(
            dir.join(d).join(file),
            "---\nRule: header.required-fields\nScope: docs/adr/ADR-1-a.md\n---\n# x\n",
        )
        .unwrap();
    }
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args(["add", "-A"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    dir
}

fn run(dir: &Path) -> (bool, serde_json::Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .arg("check")
        .current_dir(dir)
        .output()
        .unwrap();
    let json = serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
    (out.status.success(), json)
}

fn waived_by(report: &serde_json::Value) -> Option<String> {
    report["findings"]
        .as_array()?
        .iter()
        .find(|f| f["rule"] == "header.required-fields")?["waived"]
        .as_str()
        .map(str::to_string)
}

#[test]
fn a_type_declaring_the_suppression_role_suppresses_whatever_it_is_named_observed_failing() {
    let cfg = config(&type_block("exception", "docs/exception", true));
    let dir = corpus("renamed", &cfg, &[("docs/exception", "EXCEPTION-1-x.md")]);
    let (ok, report) = run(&dir);
    assert_eq!(
        waived_by(&report).as_deref(),
        Some("docs/exception/EXCEPTION-1-x.md"),
        "{report}"
    );
    assert!(ok, "the waived finding must not block: {report}");
}

#[test]
fn a_type_named_waiver_is_the_fallback_when_no_type_declares_the_role() {
    let cfg = config(&type_block("waiver", "docs/waiver", false));
    let dir = corpus("fallback", &cfg, &[("docs/waiver", "WAIVER-1-x.md")]);
    let (_, report) = run(&dir);
    assert_eq!(
        waived_by(&report).as_deref(),
        Some("docs/waiver/WAIVER-1-x.md"),
        "{report}"
    );
}

#[test]
fn a_declared_role_displaces_the_literal_name_observed_failing() {
    let cfg = config(&format!(
        "{}{}",
        type_block("exception", "docs/exception", true),
        type_block("waiver", "docs/waiver", false)
    ));
    let dir = corpus("displaced", &cfg, &[("docs/waiver", "WAIVER-1-x.md")]);
    let (ok, report) = run(&dir);
    assert_eq!(
        waived_by(&report),
        None,
        "a type merely named waiver must not suppress once another declares the role: {report}"
    );
    assert!(!ok);
    assert!(
        report["findings"]
            .as_array()
            .is_some_and(|f| f.iter().any(|f| f["rule"] == "header.required-fields")),
        "the finding must be reported, not the run refused: {report}"
    );
}

#[test]
fn two_types_declaring_the_role_are_refused_at_load() {
    let cfg = config(&format!(
        "{}{}",
        type_block("exception", "docs/exception", true),
        type_block("waiver", "docs/waiver", true)
    ));
    let dir = corpus("two", &cfg, &[]);
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .arg("check")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let text =
        String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    assert!(text.contains("suppression"), "{text}");
}

// BUG-177: `effective_requires` makes a suppression take effect only once it
// carries the declared fields.

fn declared_type(requires: Option<&str>) -> String {
    let requires = requires
        .map(|r| format!("    effective_requires: {r}\n"))
        .unwrap_or_default();
    format!(
        "  waiver:\n    dir: docs/waiver\n    role: suppression\n{requires}    required_fields: [Rule, Scope]\n    known_fields: [Reason]\n    statuses:\n      values: [Open]\n      pending: [pending]\n    header_shape: yaml-frontmatter\n"
    )
}

fn with_reason(dir: &Path, reason: Option<&str>) {
    let reason = reason.map(|r| format!("Reason: {r}\n")).unwrap_or_default();
    std::fs::write(
        dir.join("docs/waiver/WAIVER-1-x.md"),
        format!(
            "---\nRule: header.required-fields\nScope: docs/adr/ADR-1-a.md\n{reason}---\n# x\n"
        ),
    )
    .unwrap();
    assert!(Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
}

fn config_with_matches_nothing(types: &str) -> String {
    config(types).replace("rules:\n", "rules:\n  waiver.matches-nothing: warn\n")
}

fn findings_of<'a>(report: &'a serde_json::Value, rule: &str) -> Vec<&'a serde_json::Value> {
    report["findings"]
        .as_array()
        .map(|f| f.iter().filter(|f| f["rule"] == rule).collect())
        .unwrap_or_default()
}

#[test]
fn a_suppression_missing_a_required_field_does_not_suppress_and_is_reported_observed_failing() {
    let cfg = config_with_matches_nothing(&declared_type(Some("[Reason]")));
    let dir = corpus("req-missing", &cfg, &[("docs/waiver", "WAIVER-1-x.md")]);
    for (case, reason) in [
        ("absent", None),
        ("blank", Some("''")),
        ("placeholder", Some("TBD")),
        ("pending", Some("pending")),
    ] {
        with_reason(&dir, reason);
        let (ok, report) = run(&dir);
        assert_eq!(waived_by(&report), None, "{case}: {report}");
        assert!(!ok, "{case}: the target finding must block again: {report}");
        let on_waiver = findings_of(&report, "waiver.matches-nothing");
        assert_eq!(on_waiver.len(), 1, "{case}: {report}");
        assert_eq!(on_waiver[0]["file"], "docs/waiver/WAIVER-1-x.md");
        assert!(
            on_waiver[0]["message"].as_str().unwrap().contains("Reason"),
            "{case}: {report}"
        );
    }
}

#[test]
fn a_suppression_carrying_the_required_field_suppresses() {
    let cfg = config_with_matches_nothing(&declared_type(Some("[Reason]")));
    let dir = corpus("req-present", &cfg, &[("docs/waiver", "WAIVER-1-x.md")]);
    with_reason(&dir, Some("a structural exception"));
    let (ok, report) = run(&dir);
    assert_eq!(
        waived_by(&report).as_deref(),
        Some("docs/waiver/WAIVER-1-x.md"),
        "{report}"
    );
    assert!(ok, "{report}");
    assert!(findings_of(&report, "waiver.matches-nothing").is_empty());
}

#[test]
fn without_the_declaration_a_reasonless_suppression_still_suppresses() {
    let cfg = config_with_matches_nothing(&declared_type(None));
    let dir = corpus("req-default", &cfg, &[("docs/waiver", "WAIVER-1-x.md")]);
    with_reason(&dir, None);
    let (ok, report) = run(&dir);
    assert_eq!(
        waived_by(&report).as_deref(),
        Some("docs/waiver/WAIVER-1-x.md"),
        "{report}"
    );
    assert!(ok, "{report}");
}

#[test]
fn effective_requires_on_a_type_without_the_role_is_refused_at_load() {
    let cfg = config(
        "  waiver:\n    dir: docs/waiver\n    effective_requires: [Reason]\n    required_fields: [Rule, Scope, Reason]\n    header_shape: yaml-frontmatter\n",
    );
    let dir = corpus("req-norole", &cfg, &[]);
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .arg("check")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(text.contains("effective_requires"), "{text}");
}
