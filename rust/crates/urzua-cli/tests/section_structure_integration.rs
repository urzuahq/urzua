//! `section.duplicate`, `section.level-skipped` and `section.order` (MILE-104)
//! end to end: the real binary against an isolated git fixture. The rules are
//! opt-in, and each reads one key of a type's declared `sections`
//! (`unique_headings`, `no_skipped_levels`, `order`), so a type that declares
//! none of them is untouched whatever the rules table says.

mod common;

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

fn fixture(
    name: &str,
    rules: &str,
    sections: &str,
    template: Option<&str>,
    records: &[(&str, &str)],
) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-structure-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua/templates")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules: {rules}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\"]\n    header_shape: yaml-frontmatter\n{sections}"
        ),
    )
    .unwrap();
    if let Some(template) = template {
        std::fs::write(dir.join(".urzua/templates/adr.md"), template).unwrap();
    }
    for (file, content) in records {
        std::fs::write(dir.join("docs/adr").join(file), content).unwrap();
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn run(dir: &Path) -> (i32, serde_json::Value, String) {
    let output = common::urzua()
        .args(["check", "docs/"])
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    // A refusal prints its JSON error on stdout; the text is returned with stderr.
    let stderr = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed = serde_json::from_slice(&output.stdout).unwrap_or(serde_json::Value::Null);
    (output.status.code().unwrap_or(-1), parsed, stderr)
}

fn check(dir: &Path) -> (i32, serde_json::Value) {
    let (code, report, stderr) = run(dir);
    assert!(!report.is_null(), "check must print JSON: {stderr}");
    (code, report)
}

fn findings_of<'a>(report: &'a serde_json::Value, id: &str) -> Vec<&'a serde_json::Value> {
    report["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .filter(|f| f["rule"] == id)
        .collect()
}

fn population(report: &serde_json::Value, id: &str) -> (u64, u64, u64) {
    let pop = &report["rules_executed"]
        .as_array()
        .expect("rules_executed")
        .iter()
        .find(|r| r["rule"] == id)
        .unwrap_or_else(|| panic!("{id} absent from rules_executed: {report}"))["population"];
    assert_eq!(pop["unit"], "record");
    (
        pop["eligible"].as_u64().unwrap(),
        pop["examined"].as_u64().unwrap(),
        pop["out_of_scope"].as_u64().unwrap(),
    )
}

const CLEAN: &str = "---\nStatus: Accepted\n---\n\n# 1 — Clean\n\n## Context\n\nc\n\n## Decision\n\nd\n\n## References\n\nr\n";
const TWICE: &str = "---\nStatus: Accepted\n---\n\n# 2 — Twice\n\n## Context\n\nc\n\n## Open questions\n\nfirst\n\n## Decision\n\nd\n\n## Open questions\n\nsecond\n";
const NESTED_SAME: &str = "---\nStatus: Accepted\n---\n\n# 3 — Nested\n\n## Option A\n\n### Pros\n\np\n\n## Option B\n\n### Pros\n\np\n";
const NESTED_TWICE: &str = "---\nStatus: Accepted\n---\n\n# 4 — Nested twice\n\n## Option A\n\n### Pros\n\np\n\n### Pros\n\nagain\n";
const FENCED: &str =
    "---\nStatus: Accepted\n---\n\n# 5 — Fenced\n\n## Context\n\n```md\n## Context\n```\n";

const UNIQUE: &str =
    "    sections:\n      from: headings\n      depth: 2\n      unique_headings: true\n";

#[test]
fn a_heading_that_appears_twice_is_reported_at_its_second_line() {
    let dir = fixture(
        "dup",
        "{ section.duplicate: error }",
        UNIQUE,
        None,
        &[
            ("ADR-1-clean.md", CLEAN),
            ("ADR-2-twice.md", TWICE),
            ("ADR-3-nested.md", NESTED_SAME),
            ("ADR-4-nested-twice.md", NESTED_TWICE),
            ("ADR-5-fenced.md", FENCED),
        ],
    );
    let (code, report) = check(&dir);
    assert_eq!(code, 1, "{report}");
    let found = findings_of(&report, "section.duplicate");
    assert_eq!(found.len(), 2, "{report}");
    let twice = found
        .iter()
        .find(|f| f["file"].as_str().unwrap().ends_with("ADR-2-twice.md"))
        .expect("the twice record is reported");
    // `## Open questions` first at line 11, again at line 19.
    assert_eq!(twice["line"], 19, "{twice}");
    let message = twice["message"].as_str().unwrap();
    assert!(
        message.contains("Open questions") && message.contains("11"),
        "{message}"
    );
    assert!(found.iter().any(|f| f["file"]
        .as_str()
        .unwrap()
        .ends_with("ADR-4-nested-twice.md")));
    assert_eq!(population(&report, "section.duplicate"), (5, 5, 0));
}

#[test]
fn a_declared_exemption_does_not_excuse_a_broken_document() {
    let sections = "    sections:\n      from: headings\n      unique_headings: true\n      required:\n        - {name: Context}\n      exempt_statuses: [Withdrawn]\n";
    let withdrawn = TWICE.replace("Accepted", "Withdrawn");
    let dir = fixture(
        "dup-exempt",
        "{ section.duplicate: error }",
        sections,
        None,
        &[("ADR-2-twice.md", &withdrawn)],
    );
    let (code, report) = check(&dir);
    assert_eq!(code, 1, "{report}");
    assert_eq!(findings_of(&report, "section.duplicate").len(), 1);
}

#[test]
fn a_type_that_declares_no_structure_key_is_untouched() {
    let dir = fixture(
        "undeclared",
        "{ section.duplicate: error, section.level-skipped: error, section.order: error }",
        "    sections:\n      from: headings\n      depth: 2\n",
        Some("# T\n\n## Context\n"),
        &[("ADR-2-twice.md", TWICE)],
    );
    let (_, report) = check(&dir);
    for id in [
        "section.duplicate",
        "section.level-skipped",
        "section.order",
    ] {
        assert!(findings_of(&report, id).is_empty(), "{id}: {report}");
        assert_eq!(population(&report, id), (0, 0, 0), "{id}");
    }
    // Enabled and reaching no record: the census says so (0 eligible) rather than passing.
}

#[test]
fn a_type_with_no_sections_declaration_is_untouched_when_rules_are_absent() {
    let dir = fixture(
        "absent",
        "{ header.required-fields: error }",
        "",
        None,
        &[("ADR-2-twice.md", TWICE)],
    );
    let (code, report) = check(&dir);
    assert_eq!(code, 0, "{report}");
    assert!(findings_of(&report, "section.duplicate").is_empty());
}

const SKIP: &str =
    "    sections:\n      from: headings\n      depth: 2\n      no_skipped_levels: true\n";
const DEEP: &str = "---\nStatus: Accepted\n---\n\n# 6 — Deep\n\n### Orphan\n\nx\n\n## Context\n\nc\n\n### Fine\n\nok\n";
const DEEP_OK: &str = "---\nStatus: Accepted\n---\n\n# 7 — Fine\n\n## Context\n\nc\n\n### Detail\n\nok\n\n## Decision\n\nd\n";

#[test]
fn a_level_with_no_parent_above_it_is_a_skip_relative_to_the_depth() {
    let dir = fixture(
        "skip",
        "{ section.level-skipped: error }",
        SKIP,
        None,
        &[("ADR-6-deep.md", DEEP), ("ADR-7-fine.md", DEEP_OK)],
    );
    let (code, report) = check(&dir);
    assert_eq!(code, 1, "{report}");
    let found = findings_of(&report, "section.level-skipped");
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["file"]
        .as_str()
        .unwrap()
        .ends_with("ADR-6-deep.md"));
    assert_eq!(found[0]["line"], 7);
    assert!(found[0]["message"].as_str().unwrap().contains("Orphan"));
    assert_eq!(population(&report, "section.level-skipped"), (2, 2, 0));
}

#[test]
fn a_level_two_deeper_than_its_parent_is_a_skip_at_depth_three() {
    let record =
        "---\nStatus: Accepted\n---\n\n# 8 — Jump\n\n## Context\n\nc\n\n#### Too deep\n\nx\n";
    let sections =
        "    sections:\n      from: headings\n      depth: 3\n      no_skipped_levels: true\n";
    let dir = fixture(
        "skip-jump",
        "{ section.level-skipped: error }",
        sections,
        None,
        &[("ADR-8-jump.md", record)],
    );
    let (code, report) = check(&dir);
    assert_eq!(code, 1, "{report}");
    let found = findings_of(&report, "section.level-skipped");
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(found[0]["line"], 11);
}

#[test]
fn a_heading_below_the_declared_depth_is_not_read_as_a_skip() {
    // At depth 2 the tree stops at `###`: a `####` is body text, not a section.
    let record =
        "---\nStatus: Accepted\n---\n\n# 9 — Body\n\n## Context\n\nc\n\n#### Body text\n\nx\n";
    let dir = fixture(
        "skip-depth",
        "{ section.level-skipped: error }",
        SKIP,
        None,
        &[("ADR-9-body.md", record)],
    );
    let (code, report) = check(&dir);
    assert_eq!(code, 0, "{report}");
}

const ORDER: &str = "    sections:\n      from: headings\n      order: {from: template}\n";
const TEMPLATE: &str = "---\nStatus: Proposed\n---\n\n# NNNN — Title\n\n## Context\n\nc\n\n## Decision\n\nd\n\n## Consequences\n\nc\n\n## References\n\nr\n";
const IN_ORDER: &str = "---\nStatus: Accepted\n---\n\n# 1 — In order\n\n## Context\n\nc\n\n## Extra\n\ne\n\n## Decision\n\nd\n\n## References\n\nr\n";
const SWAPPED: &str = "---\nStatus: Accepted\n---\n\n# 2 — Swapped\n\n## Context\n\nc\n\n## Consequences\n\nc\n\n## Decision\n\nd\n\n## References\n\nr\n";

#[test]
fn sections_out_of_the_templates_order_are_reported() {
    let dir = fixture(
        "order",
        "{ section.order: error }",
        ORDER,
        Some(TEMPLATE),
        &[
            ("ADR-1-in-order.md", IN_ORDER),
            ("ADR-2-swapped.md", SWAPPED),
        ],
    );
    let (code, report) = check(&dir);
    assert_eq!(code, 1, "{report}");
    let found = findings_of(&report, "section.order");
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["file"]
        .as_str()
        .unwrap()
        .ends_with("ADR-2-swapped.md"));
    // `## Decision` (line 15) follows `## Consequences`, which the template puts after it.
    assert_eq!(found[0]["line"], 15);
    let message = found[0]["message"].as_str().unwrap();
    assert!(
        message.contains("Decision") && message.contains("Consequences"),
        "{message}"
    );
    assert_eq!(population(&report, "section.order"), (2, 2, 0));
}

#[test]
fn a_missing_template_is_a_finding_that_fails_under_fail_on_warning() {
    let dir = fixture(
        "order-no-template",
        "{ section.order: warn }\nfail_on: warning",
        ORDER,
        None,
        &[("ADR-2-swapped.md", SWAPPED)],
    );
    let (code, report) = check(&dir);
    assert_eq!(
        code, 1,
        "a deleted template must not switch the rule off: {report}"
    );
    let found = findings_of(&report, "section.order");
    assert_eq!(found.len(), 1, "{report}");
    let message = found[0]["message"].as_str().unwrap();
    assert!(
        message.contains(".urzua/templates/adr.md") && message.contains("does not exist"),
        "{message}"
    );
    // The record is eligible and unexamined, never passed.
    assert_eq!(population(&report, "section.order"), (1, 0, 0));
}

#[test]
fn a_template_with_no_section_is_a_finding_that_fails_under_fail_on_warning() {
    let dir = fixture(
        "order-empty-template",
        "{ section.order: warn }\nfail_on: warning",
        ORDER,
        Some("---\nStatus: Proposed\n---\n# NNNN — Title\n"),
        &[("ADR-2-swapped.md", SWAPPED)],
    );
    let (code, report) = check(&dir);
    assert_eq!(code, 1, "{report}");
    let found = findings_of(&report, "section.order");
    assert_eq!(found.len(), 1, "{report}");
    assert!(
        found[0]["message"]
            .as_str()
            .unwrap()
            .contains("holds no section"),
        "{report}"
    );
    assert_eq!(population(&report, "section.order"), (1, 0, 0));
}

#[test]
fn order_matches_heading_text_and_ignores_required_aliases() {
    let sections = "    sections:\n      from: headings\n      order: {from: template}\n      required:\n        - {name: Decision, aliases: [Outcome]}\n";
    // `Outcome` is an alias of `Decision`, but order compares heading text only,
    // so it is a section the template does not name and is ignored.
    let aliased = "---\nStatus: Accepted\n---\n\n# 3 — Aliased\n\n## Context\n\nc\n\n## Consequences\n\nc\n\n## Outcome\n\nd\n";
    let dir = fixture(
        "order-alias",
        "{ section.order: error }",
        sections,
        Some(TEMPLATE),
        &[("ADR-3-aliased.md", aliased)],
    );
    let (code, report) = check(&dir);
    assert_eq!(code, 0, "{report}");
    assert!(findings_of(&report, "section.order").is_empty(), "{report}");
    assert_eq!(population(&report, "section.order"), (1, 1, 0));
}

#[test]
fn duplicate_headings_are_compared_as_exact_trimmed_case_sensitive_text() {
    let sections = "    sections:\n      from: headings\n      unique_headings: true\n";
    let case = "---\nStatus: Accepted\n---\n\n# 4 — Case\n\n## Context\n\nc\n\n## context\n\nc\n";
    let padded =
        "---\nStatus: Accepted\n---\n\n# 5 — Padded\n\n## Context\n\nc\n\n##   Context  \n\nc\n";
    let dir = fixture(
        "dup-exact",
        "{ section.duplicate: error }",
        sections,
        None,
        &[("ADR-4-case.md", case), ("ADR-5-padded.md", padded)],
    );
    let (_, report) = check(&dir);
    let found = findings_of(&report, "section.duplicate");
    assert_eq!(
        found.len(),
        1,
        "case differs, whitespace does not: {report}"
    );
    assert!(found[0]["file"]
        .as_str()
        .unwrap()
        .ends_with("ADR-5-padded.md"));
}

#[test]
fn an_order_is_read_within_one_parent() {
    let template = "# T\n\n## Options\n\n### Pros\n\n### Cons\n\n## Decision\n";
    let sections =
        "    sections:\n      from: headings\n      depth: 2\n      order: {from: template}\n";
    let ok = "---\nStatus: Accepted\n---\n\n# 1 — Ok\n\n## Options\n\n### Pros\n\n### Cons\n\n## Decision\n";
    let bad = "---\nStatus: Accepted\n---\n\n# 2 — Bad\n\n## Options\n\n### Cons\n\n### Pros\n\n## Decision\n";
    let dir = fixture(
        "order-nested",
        "{ section.order: error }",
        sections,
        Some(template),
        &[("ADR-1-ok.md", ok), ("ADR-2-bad.md", bad)],
    );
    let (_, report) = check(&dir);
    let found = findings_of(&report, "section.order");
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["file"].as_str().unwrap().ends_with("ADR-2-bad.md"));
}

#[test]
fn a_structure_declaration_that_can_never_report_is_refused_at_load() {
    for (name, sections, needle) in [
        (
            "skip-depth-one",
            "    sections:\n      from: headings\n      no_skipped_levels: true\n",
            "no_skipped_levels",
        ),
        (
            "order-source",
            "    sections:\n      from: headings\n      order: {from: alphabet}\n",
            "order",
        ),
        (
            "unique-not-bool",
            "    sections:\n      from: headings\n      unique_headings: sometimes\n",
            "unique_headings",
        ),
    ] {
        let dir = fixture(name, "{ section.duplicate: error }", sections, None, &[]);
        let (code, _, stderr) = run(&dir);
        assert_eq!(code, 2, "{name}: {stderr}");
        assert!(stderr.contains(needle), "{name}: {stderr}");
    }
}

const DUPLICATED_SECTION: &str =
    "---\nStatus: Accepted\n---\n# ADR-1: Doubled\n\n## Context\n\ntext\n\n## Context\n\ntext\n";

/// 6.4: the four section rules enabled at `error` while no type declares the
/// key each reads passed with `ok`, exit 0 and no notice over a record with a
/// duplicated heading, and SPEC-2 said `config.scope-matches-nothing` reported it.
#[test]
fn section_rules_enabled_with_no_type_declaring_their_key_say_so() {
    let rules = "{section.duplicate: error, section.level-skipped: error, section.order: error, section.required: error}";
    let dir = fixture(
        "undeclared",
        rules,
        "",
        None,
        &[("ADR-1-doubled.md", DUPLICATED_SECTION)],
    );
    let (code, report) = check(&dir);
    assert_eq!(code, 0, "{report}");
    let said: Vec<String> = report["notices"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|n| n["subject"] == "rule-reads-nothing-declared")
        .map(|n| {
            assert_eq!(n["severity"], "warning", "{n}");
            n["message"].as_str().unwrap().to_string()
        })
        .collect();
    for (rule, key) in [
        ("section.duplicate", "sections.unique_headings"),
        ("section.level-skipped", "sections.no_skipped_levels"),
        ("section.order", "sections.order"),
        ("section.required", "sections.required"),
    ] {
        assert!(
            said.iter()
                .any(|m| m.contains(&format!("'{rule}'")) && m.contains(key)),
            "no notice for {rule}: {said:?}"
        );
    }
}

/// With `config.scope-matches-nothing` on, the same fact is its finding, which is
/// what SPEC-2 promised.
#[test]
fn config_scope_matches_nothing_reports_a_section_rule_no_type_declares() {
    let rules = "{section.duplicate: error, config.scope-matches-nothing: warn}";
    let dir = fixture(
        "undeclared-finding",
        rules,
        "",
        None,
        &[("ADR-1-doubled.md", DUPLICATED_SECTION)],
    );
    let (_, report) = check(&dir);
    let found = findings_of(&report, "config.scope-matches-nothing");
    assert_eq!(found.len(), 1, "{report}");
    let message = found[0]["message"].as_str().unwrap();
    assert!(
        message.contains("'section.duplicate'") && message.contains("sections.unique_headings"),
        "{message}"
    );
    assert_eq!(found[0]["file"], ".urzua/config.yaml");
}

/// The disclosure is for a rule nobody declared; a declaring type silences it.
#[test]
fn a_declaring_type_draws_no_undeclared_key_notice() {
    let dir = fixture(
        "declared",
        "{section.duplicate: error, config.scope-matches-nothing: warn}",
        "    sections: {from: headings, unique_headings: true}\n",
        None,
        &[("ADR-1-doubled.md", DUPLICATED_SECTION)],
    );
    let (_, report) = check(&dir);
    assert!(report["notices"]
        .as_array()
        .into_iter()
        .flatten()
        .all(|n| n["subject"] != "rule-reads-nothing-declared"));
    assert!(findings_of(&report, "config.scope-matches-nothing").is_empty());
    assert_eq!(findings_of(&report, "section.duplicate").len(), 1);
}
