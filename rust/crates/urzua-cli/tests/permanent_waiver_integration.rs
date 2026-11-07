//! MILE-193 / ADR-86: a suppression type may declare `permanent`, which says
//! per rule which reasons let a waiver without `Expires` take effect.
//!
//! Verifies: ADR-86

mod common;

use std::path::Path;
use std::process::Command;

const PERMANENT: &str = "    permanent:
      - rule: header.required-fields
        reason_field: Category
        categories:
          - {name: perf}
          - {name: decided, cite: required}
        cite_field: Justified-by
";

fn config(permanent: &str) -> String {
    format!(
        "schema_version: 2
rules:
  header.required-fields: error
  waiver.matches-nothing: warn
  pointer.resolution: error
  pointer.target-status:
    level: warn
    not_in: [Superseded]
record_types:
  adr:
    dir: docs/adr
    required_fields: [Status, Author]
    header_shape: yaml-frontmatter
  rfc:
    dir: docs/rfc
    required_fields: [Status]
    header_shape: yaml-frontmatter
  waiver:
    dir: docs/waiver
    role: suppression
    effective_requires: [Reason]
{permanent}    required_fields: [Rule, Scope, Reason]
    known_fields: [Expires, Category, Justified-by]
    pointer_fields: [Justified-by]
    narrative_fields: []
    header_shape: yaml-frontmatter
"
    )
}

fn corpus(name: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-permanent-{name}"));
    for d in ["docs/adr", "docs/rfc", "docs/waiver", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    // ADR-1 lacks Author: the live finding the waiver covers.
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        "---\nStatus: Accepted\n---\n# 1 - A\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/rfc/RFC-35-x.md"),
        "---\nStatus: Accepted\n---\n# 35 - X\n",
    )
    .unwrap();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    dir
}

/// Writes the one waiver, `extra` being its remaining header lines.
fn waiver(dir: &Path, extra: &str) {
    std::fs::write(
        dir.join("docs/waiver/WAIVER-1-x.md"),
        format!(
            "---\nRule: header.required-fields\nScope: docs/adr/ADR-1-a.md\nReason: a stated reason\n{extra}---\n# x\n"
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

fn run(dir: &Path) -> (bool, serde_json::Value) {
    let out = common::urzua()
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

fn ineffective(report: &serde_json::Value) -> Vec<String> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| {
            f["rule"] == "waiver.matches-nothing"
                && f["message"].as_str().unwrap().contains("not in effect")
        })
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect()
}

fn permanent_notice(report: &serde_json::Value) -> Option<String> {
    report["notices"]
        .as_array()?
        .iter()
        .find(|n| n["subject"] == "permanent-waivers")?["message"]
        .as_str()
        .map(str::to_string)
}

#[test]
fn a_no_expiry_waiver_with_a_listed_category_is_waived() {
    let dir = corpus("valid", &config(PERMANENT));
    waiver(&dir, "Category: perf\n");
    let (ok, report) = run(&dir);
    assert_eq!(
        waived_by(&report).as_deref(),
        Some("docs/waiver/WAIVER-1-x.md"),
        "{report}"
    );
    assert!(ok, "{report}");
    assert!(ineffective(&report).is_empty(), "{report}");
}

#[test]
fn a_no_expiry_waiver_with_an_unlisted_category_is_ineffective_naming_the_field() {
    let dir = corpus("unlisted", &config(PERMANENT));
    waiver(&dir, "Category: whim\n");
    let (ok, report) = run(&dir);
    assert_eq!(waived_by(&report), None, "{report}");
    assert!(!ok, "the finding must block again: {report}");
    let messages = ineffective(&report);
    assert_eq!(messages.len(), 1, "{report}");
    assert!(
        messages[0].contains("Category") && messages[0].contains("whim"),
        "{}",
        messages[0]
    );
    assert!(
        !messages[0].contains("effective_requires"),
        "the message must name the unmet requirement, not hard-code effective_requires: {}",
        messages[0]
    );
}

#[test]
fn a_no_expiry_waiver_without_a_category_is_ineffective() {
    let dir = corpus("nocategory", &config(PERMANENT));
    waiver(&dir, "");
    let (ok, report) = run(&dir);
    assert_eq!(waived_by(&report), None, "{report}");
    assert!(!ok);
    let messages = ineffective(&report);
    assert_eq!(messages.len(), 1, "{report}");
    assert!(messages[0].contains("Category"), "{}", messages[0]);
}

#[test]
fn a_category_requiring_a_citation_needs_one_that_resolves() {
    let dir = corpus("cite", &config(PERMANENT));

    waiver(&dir, "Category: decided\n");
    let (ok, report) = run(&dir);
    assert_eq!(waived_by(&report), None, "{report}");
    assert!(!ok);
    let messages = ineffective(&report);
    assert_eq!(messages.len(), 1, "{report}");
    assert!(messages[0].contains("Justified-by"), "{}", messages[0]);

    waiver(&dir, "Category: decided\nJustified-by: RFC-999\n");
    let (_, report) = run(&dir);
    assert_eq!(waived_by(&report), None, "an unresolved citation: {report}");
    assert!(
        ineffective(&report)[0].contains("RFC-999"),
        "{}",
        ineffective(&report)[0]
    );

    waiver(&dir, "Category: decided\nJustified-by: RFC-35\n");
    let (ok, report) = run(&dir);
    assert_eq!(
        waived_by(&report).as_deref(),
        Some("docs/waiver/WAIVER-1-x.md"),
        "{report}"
    );
    assert!(ok, "{report}");
    assert!(ineffective(&report).is_empty(), "{report}");
}

#[test]
fn a_waiver_with_an_expiry_is_unchanged_by_the_declaration() {
    let dir = corpus("expiry", &config(PERMANENT));
    waiver(&dir, "Expires: 2099-01-01\n");
    let (ok, report) = run(&dir);
    assert_eq!(
        waived_by(&report).as_deref(),
        Some("docs/waiver/WAIVER-1-x.md"),
        "{report}"
    );
    assert!(ok, "{report}");
    assert!(ineffective(&report).is_empty(), "{report}");
}

#[test]
fn declaring_permanent_closes_permanence_for_a_rule_it_does_not_list() {
    let cfg = config(PERMANENT);
    let dir = corpus("unlisted-rule", &cfg);
    std::fs::write(
        dir.join("docs/waiver/WAIVER-1-x.md"),
        "---\nRule: pointer.resolution\nScope: '*'\nReason: r\nCategory: perf\n---\n# x\n",
    )
    .unwrap();
    assert!(Command::new("git")
        .args(["add", "-A"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    let (_, report) = run(&dir);
    let messages = ineffective(&report);
    assert_eq!(messages.len(), 1, "{report}");
    assert!(
        messages[0].contains("pointer.resolution") && messages[0].contains("permanent"),
        "{}",
        messages[0]
    );
}

#[test]
fn undeclared_behaves_as_today() {
    let dir = corpus("undeclared", &config(""));
    // No Category, no Expires, any rule: allowed, as before.
    waiver(&dir, "");
    let (ok, report) = run(&dir);
    assert_eq!(
        waived_by(&report).as_deref(),
        Some("docs/waiver/WAIVER-1-x.md"),
        "{report}"
    );
    assert!(ok, "{report}");
    assert!(ineffective(&report).is_empty(), "{report}");
    assert_eq!(permanent_notice(&report), None, "{report}");
}

#[test]
fn the_report_counts_permanent_waivers_by_category() {
    let dir = corpus("count", &config(PERMANENT));
    waiver(&dir, "Category: perf\n");
    let (_, report) = run(&dir);
    let message = permanent_notice(&report).expect("a notice when permanent is declared");
    assert!(
        message.contains("1 permanent waiver") && message.contains("perf: 1"),
        "{message}"
    );
}

#[test]
fn an_ineffective_waiver_is_not_counted_as_permanent() {
    let dir = corpus("count-ineffective", &config(PERMANENT));
    waiver(&dir, "Category: whim\n");
    let (_, report) = run(&dir);
    let message = permanent_notice(&report).expect("a notice when permanent is declared");
    assert!(message.contains("0 permanent waivers"), "{message}");
}

#[test]
fn a_permanent_waiver_whose_finding_disappears_is_reported_as_matching_nothing() {
    let dir = corpus("stale", &config(PERMANENT));
    waiver(&dir, "Category: perf\n");
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        "---\nStatus: Accepted\nAuthor: a\n---\n# 1 - A\n",
    )
    .unwrap();
    let (_, report) = run(&dir);
    let stale: Vec<_> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| {
            f["rule"] == "waiver.matches-nothing"
                && f["message"]
                    .as_str()
                    .unwrap()
                    .contains("matches no finding")
        })
        .collect();
    assert_eq!(stale.len(), 1, "{report}");
}

#[test]
fn a_cited_record_that_becomes_superseded_is_reported() {
    let dir = corpus("superseded", &config(PERMANENT));
    std::fs::write(
        dir.join("docs/rfc/RFC-35-x.md"),
        "---\nStatus: Superseded\n---\n# 35 - X\n",
    )
    .unwrap();
    waiver(&dir, "Category: decided\nJustified-by: RFC-35\n");
    let (_, report) = run(&dir);
    assert!(
        report["findings"].as_array().unwrap().iter().any(|f| {
            f["rule"] == "pointer.target-status" && f["file"] == "docs/waiver/WAIVER-1-x.md"
        }),
        "{report}"
    );
}

#[test]
fn a_permanent_declaration_that_is_not_well_formed_is_refused_at_load() {
    let bad = [
        // unknown rule
        (
            PERMANENT.replace("header.required-fields", "no.such-rule"),
            "no.such-rule",
        ),
        // undeclared reason field
        (
            PERMANENT.replace("reason_field: Category", "reason_field: Nope"),
            "Nope",
        ),
        // empty category list
        (
            "    permanent:\n      - {rule: header.required-fields, reason_field: Category, categories: []}\n"
                .to_string(),
            "categories",
        ),
        // duplicated category
        (
            PERMANENT.replace("{name: decided, cite: required}", "{name: perf}"),
            "twice",
        ),
        // cite_field that is not a pointer field
        (
            PERMANENT.replace("cite_field: Justified-by", "cite_field: Expires"),
            "Expires",
        ),
        // a category requiring a citation with no cite_field
        (
            PERMANENT.replace("        cite_field: Justified-by\n", ""),
            "cite_field",
        ),
    ];
    for (i, (permanent, expected)) in bad.iter().enumerate() {
        let dir = corpus(&format!("bad-{i}"), &config(permanent));
        let out = common::urzua()
            .arg("check")
            .current_dir(&dir)
            .output()
            .unwrap();
        assert!(!out.status.success(), "case {i}:\n{permanent}");
        let text = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        assert!(
            text.contains("permanent") && text.contains(expected),
            "case {i}: {text}"
        );
    }
}
