//! `MILE-335`, `MILE-326` and `MILE-327`: a checklist (`statuses.delivery.items`)
//! and the status requirements (`statuses.require`, `statuses.forbid`) are
//! declared once and report without a second switch, a checklist needs no
//! evidence field it does not read, its errors say what to write, and a finding
//! about a field with no value says which it is and what may go there.
//!
//! Verifies: MILE-335, MILE-326, MILE-327

mod common;

use std::path::Path;

const SMALL_CHECKLIST: &str = "      delivery:
        delivered_tiers: [Implemented, Verified]
        unevidenced_tiers: [\"Not started\"]
        not_done: [Open, InProgress]
        set_status: Done
        items:
          section: Acceptance
          clause_separator: \" — \"
          evidence_marker: \"evidence:\"
          waiver_marker: \"waived:\"
          partial_status: InProgress
          waiver_categories: [{name: not-applicable}]
";

fn config(known: &str, statuses_extra: &str, top: &str) -> String {
    format!(
        "schema_version: 2
{top}rules:
  header.required-fields: warn
record_types:
  adr:
    dir: docs/adr
    prefix: ADR
    required_fields: [Status]
    header_shape: yaml-frontmatter
  task:
    dir: docs/task
    prefix: TASK
    required_fields: [Status, Kind, Stable-Id]
    header_shape: yaml-frontmatter
    known_fields: {known}
    field_values: {{Kind: [bug, feature]}}
    statuses:
      values: [Open, InProgress, Done, WontDo]
      success: [Done]
      failure: [WontDo]
{statuses_extra}    sections: {{from: headings, depth: 2, required: [{{name: Acceptance}}]}}
"
    )
}

fn record(status: &str, kind: &str, items: &str) -> String {
    format!(
        "---\nStatus: {status}\nKind: {kind}\nStable-Id: 01JTASK000000000000000001\n---\n# 1 - T\n\n## Acceptance\n\n{items}"
    )
}

fn repo(label: &str, config: &str, records: &[(&str, String)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-checklist-{label}"));
    for d in ["docs/task", "docs/adr", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-use-x.md"),
        "---\nStatus: Accepted\n---\n# 1 - Use X\n",
    )
    .unwrap();
    for (name, text) in records {
        std::fs::write(dir.join("docs/task").join(name), text).unwrap();
    }
    common::git(&dir, &["init", "-q"]);
    common::git(&dir, &["add", "-A"]);
    dir
}

fn run(dir: &Path, args: &[&str]) -> (i32, serde_json::Value) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    let json = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "json: {e}\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    (out.status.code().unwrap(), json)
}

fn findings<'a>(report: &'a serde_json::Value, rule: &str) -> Vec<&'a serde_json::Value> {
    report["findings"]
        .as_array()
        .map(|a| a.iter().filter(|f| f["rule"] == rule).collect())
        .unwrap_or_default()
}

fn rule_status(report: &serde_json::Value, rule: &str) -> String {
    report["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["rule"] == rule)
        .map(|e| e["status"].as_str().unwrap().to_string())
        .unwrap_or_else(|| "absent".to_string())
}

const CHECKLIST_RULES: [&str; 3] = [
    "status.delivery-agreement",
    "status.done-items-shape",
    "status.waived-item-stale",
];

// ------------------------------------------------ no evidence field needed

#[test]
fn a_checklist_loads_without_the_evidence_field_in_known_fields() {
    let dir = repo(
        "no-evidence-field",
        &config("[]", SMALL_CHECKLIST, ""),
        &[(
            "TASK-1-t.md",
            record("Done", "bug", "- Built. — evidence: code:src/a.rs\n"),
        )],
    );
    let (code, report) = run(&dir, &["check"]);
    assert_eq!(code, 0, "{report}");
    assert!(report.get("error").is_none(), "{report}");
    assert_eq!(rule_status(&report, "status.delivery-agreement"), "ran");
    assert!(
        findings(&report, "status.delivery-agreement").is_empty(),
        "an item resolved by its own clause needs no header field: {report}"
    );
}

#[test]
fn a_delivery_without_items_still_reads_the_evidence_field() {
    let no_items = SMALL_CHECKLIST.split("        items:").next().unwrap();
    let dir = repo("no-items", &config("[]", no_items, ""), &[]);
    let (code, report) = run(&dir, &["check"]);
    assert_eq!(code, 2, "{report}");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("evidence field 'Realized-by'"),
        "{report}"
    );
}

// ---------------------------------------------------- on by declaration

#[test]
fn the_checklist_rules_are_on_where_a_checklist_is_declared_and_name_nothing() {
    let dir = repo(
        "by-declaration",
        &config("[]", SMALL_CHECKLIST, ""),
        &[(
            "TASK-1-t.md",
            record(
                "Done",
                "bug",
                "- Built.\n- Tested. — evidence: code:src/a.rs\n",
            ),
        )],
    );
    let (code, report) = run(&dir, &["check"]);
    assert_eq!(code, 1, "{report}");
    for rule in CHECKLIST_RULES {
        assert_eq!(rule_status(&report, rule), "ran", "{rule}");
    }
    let agreement = findings(&report, "status.delivery-agreement");
    assert_eq!(agreement[0]["severity"], "error", "{report}");
    assert!(
        agreement[0]["message"].as_str().unwrap().contains("Built."),
        "{report}"
    );
}

#[test]
fn a_configuration_naming_a_checklist_rule_keeps_its_level() {
    let named = config("[]", SMALL_CHECKLIST, "").replace(
        "rules:\n",
        "rules:\n  status.done-items-shape: info\n  status.waived-item-stale: warn\n",
    );
    let dir = repo(
        "named-level",
        &named,
        &[(
            "TASK-1-t.md",
            record(
                "Open",
                "bug",
                "- Free text. — evidence: see the pull request\n",
            ),
        )],
    );
    let (_, report) = run(&dir, &["check"]);
    let shape = findings(&report, "status.done-items-shape");
    assert_eq!(shape.len(), 1, "{report}");
    assert_eq!(shape[0]["severity"], "info", "{report}");
}

#[test]
fn a_checklist_with_its_agreement_rule_turned_off_is_still_refused_at_load() {
    let named = config("[]", SMALL_CHECKLIST, "")
        .replace("rules:\n", "rules:\n  status.delivery-agreement: off\n");
    let dir = repo("agreement-off", &named, &[]);
    let (code, report) = run(&dir, &["check"]);
    assert_eq!(code, 2, "{report}");
    let error = report["error"].as_str().unwrap();
    assert!(
        error.contains("status.delivery-agreement") && error.contains("turned off"),
        "{error}"
    );
}

#[test]
fn a_configuration_declaring_no_checklist_lists_the_checklist_rules_not_enabled() {
    let plain = config("[]", "", "");
    let dir = repo(
        "no-checklist",
        &plain,
        &[("TASK-1-t.md", record("Done", "bug", "- x\n"))],
    );
    let (_, report) = run(&dir, &["check"]);
    for rule in CHECKLIST_RULES {
        assert_eq!(rule_status(&report, rule), "not-enabled", "{rule}");
    }
}

const REQUIRE: &str = "      require:
        - {when: [Done], fields: [Kind]}
        - {when_field: {Kind: [bug]}, fields: [Severity]}
      forbid:
        - {when: [Open], fields: [Closed-on]}
";

fn requirement_config(rules: &str) -> String {
    config("[Severity, Closed-on]", REQUIRE, "").replace("rules:\n", &format!("rules:\n{rules}"))
}

#[test]
fn a_declared_requirement_and_prohibition_report_with_no_rule_named() {
    let dir = repo(
        "require",
        &requirement_config(""),
        &[(
            "TASK-1-t.md",
            "---\nStatus: Open\nKind: bug\nStable-Id: 01JTASK000000000000000001\nClosed-on: 2026-01-01\n---\n# 1 - T\n\n## Acceptance\n\n- x\n"
                .to_string(),
        )],
    );
    let (_, report) = run(&dir, &["check"]);
    for rule in [
        "status.required-field-missing",
        "status.forbidden-field-present",
    ] {
        assert_eq!(rule_status(&report, rule), "ran", "{rule}");
        let found = findings(&report, rule);
        assert_eq!(found.len(), 1, "{rule}: {report}");
        assert_eq!(found[0]["severity"], "error");
    }
}

#[test]
fn a_named_requirement_rule_keeps_its_level_and_no_entry_reports_as_before() {
    let dir = repo(
        "require-named",
        &requirement_config("  status.required-field-missing: warn\n"),
        &[("TASK-1-t.md", record("Open", "bug", "- x\n"))],
    );
    let (_, report) = run(&dir, &["check"]);
    let found = findings(&report, "status.required-field-missing");
    assert_eq!(found[0]["severity"], "warning", "{report}");

    let plain = repo(
        "no-entries",
        &config("[]", "", ""),
        &[("TASK-1-t.md", record("Open", "bug", "- x\n"))],
    );
    let (_, report) = run(&plain, &["check"]);
    for rule in [
        "status.required-field-missing",
        "status.forbidden-field-present",
    ] {
        assert_eq!(rule_status(&report, rule), "not-enabled", "{rule}");
    }
}

#[test]
fn doctor_warns_of_a_declared_requirement_whose_rule_is_off() {
    let dir = repo(
        "doctor-off",
        &requirement_config("  status.required-field-missing: off\n"),
        &[],
    );
    let (_, report) = run(&dir, &["doctor"]);
    let checks = report["checks"].as_array().unwrap();
    let hit: Vec<_> = checks
        .iter()
        .filter(|c| c["check"] == "declared-entry-rule-off")
        .collect();
    assert_eq!(hit.len(), 1, "{report}");
    assert_eq!(hit[0]["status"], "warn");
    let message = hit[0]["message"].as_str().unwrap();
    assert!(
        message.contains("statuses.require") && message.contains("status.required-field-missing"),
        "{message}"
    );

    let on = repo("doctor-on", &requirement_config(""), &[]);
    let (_, report) = run(&on, &["doctor"]);
    assert!(
        !report["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["check"] == "declared-entry-rule-off"),
        "{report}"
    );
}

// ------------------------------------------------------ evidence clauses

fn done_cause(clause_lines: &str, statuses_extra: &str, top: &str) -> String {
    let dir = repo(
        "cause",
        &config("[]", statuses_extra, top),
        &[("TASK-1-t.md", record("Done", "bug", clause_lines))],
    );
    let (_, report) = run(&dir, &["check"]);
    let found = findings(&report, "status.delivery-agreement");
    assert!(!found.is_empty(), "{report}");
    found[0]["message"].as_str().unwrap().to_string()
}

#[test]
fn a_shape_error_lists_the_categories_a_locator_may_carry() {
    let dir = repo(
        "categories",
        &config("[]", SMALL_CHECKLIST, ""),
        &[(
            "TASK-1-t.md",
            record("Open", "bug", "- Free. — evidence: see the pull request\n"),
        )],
    );
    let (_, report) = run(&dir, &["check"]);
    let shape = findings(&report, "status.done-items-shape");
    let message = shape[0]["message"].as_str().unwrap();
    assert!(
        message.contains("the category is one of spec, code, test, record")
            && message.contains("`embodiment.tiers`"),
        "{message}"
    );
}

#[test]
fn a_locator_naming_a_record_by_identifier_is_told_to_cite_its_path() {
    let message = done_cause("- Decided. — evidence: spec:ADR-1\n", SMALL_CHECKLIST, "");
    assert!(
        message.contains("locator ADR-1 does not exist: it was read as a path")
            && message.contains("`spec:docs/adr/ADR-1-use-x.md`"),
        "{message}"
    );
}

#[test]
fn a_locator_that_did_not_resolve_says_it_was_read_as_a_path() {
    let message = done_cause(
        "- Built. — evidence: code:src/nope.rs\n",
        SMALL_CHECKLIST,
        "",
    );
    assert!(
        message.contains("locator src/nope.rs does not exist: it was read as a path"),
        "{message}"
    );
}

#[test]
fn a_cited_record_whose_tier_does_not_count_names_the_key_that_would_count_it() {
    let message = done_cause(
        "- Decided. — evidence: spec:docs/adr/ADR-1-use-x.md\n",
        SMALL_CHECKLIST,
        "",
    );
    assert!(
        message.contains(
            "resolves only to Specified, which is not in resolving_tiers (Implemented, Verified)"
        ) && message.contains("`record_types.task.statuses.delivery.items.resolving_tiers`"),
        "{message}"
    );
}

const RECORD_TIER: &str = "embodiment:
  drift: Drift detected
  tiers:
    - {state: Verified, evidence: test, claims_built: true}
    - {state: Implemented, evidence: code, claims_built: true}
    - {state: Decided, evidence: record}
    - {state: Specified, evidence: spec}
  none: Not started
";

#[test]
fn a_record_is_cited_under_the_record_category_once_a_tier_declares_it() {
    let items = SMALL_CHECKLIST.replace(
        "          partial_status: InProgress\n",
        "          partial_status: InProgress\n          resolving_tiers: [Implemented, Verified, Decided]\n",
    );
    let dir = repo(
        "record-category",
        &config("[]", &items, RECORD_TIER),
        &[(
            "TASK-1-t.md",
            record(
                "Done",
                "bug",
                "- Built. — evidence: code:src/a.rs\n- Decided. — evidence: record:docs/adr/ADR-1-use-x.md\n",
            ),
        )],
    );
    let (_, report) = run(&dir, &["check"]);
    assert!(report.get("error").is_none(), "{report}");
    assert!(
        findings(&report, "status.delivery-agreement").is_empty()
            && findings(&report, "status.done-items-shape").is_empty(),
        "{report}"
    );
}

#[test]
fn a_record_category_locator_that_is_not_a_record_does_not_resolve() {
    let items = SMALL_CHECKLIST.replace(
        "          partial_status: InProgress\n",
        "          partial_status: InProgress\n          resolving_tiers: [Implemented, Verified, Decided]\n",
    );
    let message = done_cause(
        "- Built. — evidence: record:src/a.rs\n",
        &items,
        RECORD_TIER,
    );
    assert!(
        message.contains("under `record:` but is not a record of any declared type"),
        "{message}"
    );
}

// --------------------------------------------------------- load errors

#[test]
fn an_incomplete_checklist_declaration_names_the_missing_key_and_shows_a_valid_block() {
    for key in ["partial_status", "clause_separator", "evidence_marker"] {
        let broken: String = SMALL_CHECKLIST
            .lines()
            .filter(|l| !l.trim_start().starts_with(&format!("{key}:")))
            .map(|l| format!("{l}\n"))
            .collect();
        let dir = repo("incomplete", &config("[]", &broken, ""), &[]);
        let (code, report) = run(&dir, &["check"]);
        assert_eq!(code, 2, "{report}");
        let error = report["error"].as_str().unwrap();
        assert!(
            error.contains(&format!("`statuses.delivery.items.{key}` is missing"))
                && error.contains("delivered_tiers: [Implemented]")
                && error.contains("sections: {from: headings"),
            "{error}"
        );
    }
    let delivery_key: String = SMALL_CHECKLIST
        .lines()
        .filter(|l| !l.trim_start().starts_with("set_status:"))
        .map(|l| format!("{l}\n"))
        .collect();
    let dir = repo("incomplete-delivery", &config("[]", &delivery_key, ""), &[]);
    let (_, report) = run(&dir, &["check"]);
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("`statuses.delivery.set_status` is missing"),
        "{report}"
    );
}

#[test]
fn a_section_the_type_does_not_require_shows_the_block_that_declares_it() {
    let cfg = config("[]", SMALL_CHECKLIST, "").replace(
        "required: [{name: Acceptance}]",
        "required: [{name: Context}]",
    );
    let dir = repo("section-not-required", &cfg, &[]);
    let (_, report) = run(&dir, &["check"]);
    let error = report["error"].as_str().unwrap();
    assert!(
        error.contains("does not require") && error.contains("sections: {from: headings"),
        "{error}"
    );
}

// ------------------------------------------ an absent field or an empty one

fn field_config(rules: &str) -> String {
    config("[Severity, Closed-on]", REQUIRE, "").replace(
        "rules:\n",
        &format!("rules:\n  field.quality: error\n  status.required-field-missing: error\n{rules}"),
    )
}

fn header_record(header: &str) -> String {
    format!(
        "---\n{header}Stable-Id: 01JTASK000000000000000001\n---\n# 1 - T\n\n## Acceptance\n\n- x\n"
    )
}

fn message_for<'a>(
    report: &'a serde_json::Value,
    rule: &str,
    field: &str,
) -> &'a serde_json::Value {
    findings(report, rule)
        .into_iter()
        .find(|f| f["suggested_action"]["target"]["field"] == field)
        .unwrap_or_else(|| panic!("no {rule} finding on {field}: {report}"))
}

#[test]
fn field_quality_says_missing_for_an_absent_field_and_empty_for_a_present_one() {
    let dir = repo(
        "quality-words",
        &field_config(""),
        &[
            ("TASK-1-a.md", header_record("Status: Open\n")),
            ("TASK-2-b.md", header_record("Status: Open\nKind:\n")),
        ],
    );
    let (_, report) = run(&dir, &["check"]);
    let found = findings(&report, "field.quality");
    let absent = found
        .iter()
        .find(|f| f["file"].as_str().unwrap().contains("TASK-1"))
        .unwrap();
    let empty = found
        .iter()
        .find(|f| f["file"].as_str().unwrap().contains("TASK-2"))
        .unwrap();
    assert_eq!(
        absent["message"],
        "field 'Kind' is missing from the header (allowed: bug, feature)"
    );
    assert_eq!(
        empty["message"],
        "field 'Kind' is empty -- not a real, present value (allowed: bug, feature)"
    );
}

#[test]
fn the_suggested_action_of_a_finding_about_no_value_carries_the_declared_values() {
    let dir = repo(
        "quality-action",
        &field_config(""),
        &[("TASK-1-a.md", header_record("Status: Open\nKind:\n"))],
    );
    let (_, report) = run(&dir, &["check"]);
    let finding = message_for(&report, "field.quality", "Kind");
    assert_eq!(
        finding["suggested_action"]["value"],
        serde_json::json!(["bug", "feature"]),
        "{finding}"
    );
}

#[test]
fn a_field_without_a_declared_vocabulary_keeps_its_message_and_carries_no_values() {
    let dir = repo(
        "quality-no-vocab",
        &field_config(""),
        &[(
            "TASK-1-a.md",
            header_record("Status: Open\nKind: bug\nSeverity:\n"),
        )],
    );
    let (_, report) = run(&dir, &["check"]);
    let finding = message_for(&report, "status.required-field-missing", "Severity");
    assert_eq!(
        finding["message"],
        "Kind is bug, which requires 'Severity', but it is empty -- not a real, present value"
    );
    assert!(
        finding["suggested_action"].get("value").is_none(),
        "{finding}"
    );
}

#[test]
fn the_requirement_rule_says_missing_for_an_absent_field_and_names_the_allowed_values() {
    let dir = repo(
        "require-words",
        &field_config(""),
        &[
            ("TASK-1-a.md", header_record("Status: Done\n")),
            ("TASK-2-b.md", header_record("Status: Done\nKind:\n")),
        ],
    );
    let (_, report) = run(&dir, &["check"]);
    let found = findings(&report, "status.required-field-missing");
    let message = |n: &str| {
        found
            .iter()
            .find(|f| f["file"].as_str().unwrap().contains(n))
            .unwrap()["message"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert_eq!(
        message("TASK-1"),
        "Status is Done, which requires 'Kind', but it is missing from the header (allowed: bug, feature)"
    );
    assert_eq!(
        message("TASK-2"),
        "Status is Done, which requires 'Kind', but it is empty -- not a real, present value (allowed: bug, feature)"
    );
    let action = &message_for(&report, "status.required-field-missing", "Kind")["suggested_action"];
    assert_eq!(
        action["value"],
        serde_json::json!(["bug", "feature"]),
        "{action}"
    );
}

// -------------------------------- a declaration is a configuration that governs

fn bare_config() -> String {
    config("[]", SMALL_CHECKLIST, "").replace("  header.required-fields: warn\n", "")
}

#[test]
fn a_clean_run_of_a_configuration_that_names_no_rule_is_ok_not_not_run() {
    let dir = repo(
        "clean-ok",
        &bare_config(),
        &[(
            "TASK-1-t.md",
            record(
                "Done",
                "bug",
                "- Built. — evidence: code:src/a.rs\n- Not here. — waived: not-applicable -- no second platform\n",
            ),
        )],
    );
    let (code, report) = run(&dir, &["check"]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["status"], "ok", "{report}");
    let notices = report["notices"].as_array().cloned().unwrap_or_default();
    assert!(
        notices
            .iter()
            .any(|n| n["message"].as_str().unwrap().contains("not-applicable 1")),
        "the waived item is listed though no rule is named: {report}"
    );
}

#[test]
fn fix_repairs_a_checklist_status_with_no_rule_named() {
    let dir = repo(
        "fix-default",
        &bare_config(),
        &[(
            "TASK-1-t.md",
            record("Open", "bug", "- Built. — evidence: code:src/a.rs\n"),
        )],
    );
    let (_, report) = run(&dir, &["fix"]);
    assert_eq!(report["status"], "repairs-available", "{report}");
    assert_eq!(report["repairs"][0]["computed_value"], "Done", "{report}");
}

// ------------------------------------ a rule applied by declaration must read a record

const REQUIRE_ONLY: &str = "schema_version: 2
record_types:
  task:
    dir: docs/task
    prefix: TASK
    required_fields: [Status, Kind]
    header_shape: yaml-frontmatter
    statuses:
      values: [Open, Done]
      success: [Done]
      require:
        - {when: [Done], fields: [Kind]}
  note:
    dir: docs/note
    prefix: NOTE
    required_fields: [Status]
    header_shape: yaml-frontmatter
";

fn note_repo(label: &str, task: Option<String>) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-checklist-{label}"));
    for d in ["docs/task", "docs/note", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    std::fs::write(dir.join(".urzua/config.yaml"), REQUIRE_ONLY).unwrap();
    std::fs::write(
        dir.join("docs/note/NOTE-1-n.md"),
        "---\nStatus: Open\n---\n# 1 - N\n",
    )
    .unwrap();
    if let Some(task) = task {
        std::fs::write(dir.join("docs/task/TASK-1-t.md"), task).unwrap();
    }
    common::git(&dir, &["init", "-q"]);
    common::git(&dir, &["add", "-A"]);
    dir
}

#[test]
fn a_rule_applied_by_declaration_that_read_no_record_leaves_the_run_not_run() {
    let dir = note_repo("declared-reads-nothing", None);
    let (code, report) = run(&dir, &["check"]);
    assert_eq!(report["status"], "not-run", "{report}");
    assert_eq!(code, 2, "{report}");
    assert_eq!(report["records_read_by_any_rule"], 0, "{report}");
}

#[test]
fn a_rule_applied_by_declaration_that_read_a_record_makes_a_clean_run_ok() {
    let dir = note_repo(
        "declared-reads-one",
        Some("---\nStatus: Done\nKind: bug\n---\n# 1 - T\n".to_string()),
    );
    let (code, report) = run(&dir, &["check"]);
    assert_eq!(report["status"], "ok", "{report}");
    assert_eq!(code, 0, "{report}");
}

// ---------------------------- a selected rule that applies nowhere says why

#[test]
fn a_selected_rule_that_applies_by_declaration_names_its_own_declaration() {
    let dir = repo(
        "selection-notice",
        &config("[]", "", ""),
        &[("TASK-1-t.md", record("Open", "bug", "- x\n"))],
    );
    let want = [
        ("status.delivery-agreement", "`statuses.delivery.items`"),
        ("status.done-items-shape", "`statuses.delivery.items`"),
        ("status.waived-item-stale", "`statuses.delivery.items`"),
        ("status.required-field-missing", "`statuses.require`"),
        ("status.forbidden-field-present", "`statuses.forbid`"),
    ];
    for (rule, key) in want {
        let selection = format!("header.required-fields,{rule}");
        let (_, report) = run(&dir, &["check", "--rules", &selection]);
        let notice = report["notices"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["subject"] == rule)
            .unwrap_or_else(|| panic!("no notice for {rule}: {report}"));
        let message = notice["message"].as_str().unwrap();
        assert!(
            message.contains(&format!("is on only where {key} is declared")),
            "{rule}: {message}"
        );
    }
}

// ------------------------- a suggested repair is one that loads

#[test]
fn a_record_locator_with_no_tier_for_the_category_points_at_the_embodiment_tiers() {
    let message = done_cause(
        "- Decided. — evidence: record:docs/adr/ADR-1-use-x.md\n",
        SMALL_CHECKLIST,
        "",
    );
    assert!(
        message.contains("no tier in `embodiment.tiers` declares `evidence: record`")
            && message.contains("declare one and add its state to"),
        "{message}"
    );
    assert!(
        !message.contains("add 'Not started'"),
        "'Not started' is not a tier a resolving_tiers entry may name: {message}"
    );
}

#[test]
fn the_repair_a_finding_suggests_loads() {
    let spec_message = done_cause(
        "- Decided. — evidence: spec:docs/adr/ADR-1-use-x.md\n",
        SMALL_CHECKLIST,
        "",
    );
    assert!(
        spec_message.contains("add 'Specified' to"),
        "{spec_message}"
    );
    let repaired = SMALL_CHECKLIST.replace(
        "          partial_status: InProgress\n",
        "          partial_status: InProgress\n          resolving_tiers: [Implemented, Verified, Specified]\n",
    );
    let dir = repo(
        "repair-loads",
        &config("[]", &repaired, ""),
        &[(
            "TASK-1-t.md",
            record("Done", "bug", "- Built. — evidence: code:src/a.rs\n- Decided. — evidence: spec:docs/adr/ADR-1-use-x.md\n"),
        )],
    );
    let (code, report) = run(&dir, &["check"]);
    assert_eq!(code, 0, "{report}");
    assert!(report.get("error").is_none(), "{report}");

    let no_tier = done_cause(
        "- Decided. — evidence: record:docs/adr/ADR-1-use-x.md\n",
        SMALL_CHECKLIST,
        "",
    );
    assert!(no_tier.contains("declare one"), "{no_tier}");
    let items = SMALL_CHECKLIST.replace(
        "          partial_status: InProgress\n",
        "          partial_status: InProgress\n          resolving_tiers: [Implemented, Verified, Decided]\n",
    );
    let dir = repo(
        "repair-loads-record",
        &config("[]", &items, RECORD_TIER),
        &[(
            "TASK-1-t.md",
            record("Done", "bug", "- Built. — evidence: code:src/a.rs\n- Decided. — evidence: record:docs/adr/ADR-1-use-x.md\n"),
        )],
    );
    let (code, report) = run(&dir, &["check"]);
    assert_eq!(code, 0, "{report}");
}

#[test]
fn a_record_locator_in_a_header_must_name_a_record_as_an_item_one_must() {
    let tiers = RECORD_TIER;
    let cfg = config("[Realized-by]", "", tiers)
        .replace("rules:\n", "rules:\n  embodiment.locator-exists: error\n");
    let header = |locator: &str| {
        format!(
            "---\nStatus: Open\nKind: bug\nStable-Id: 01JTASK000000000000000001\nRealized-by: {locator}\n---\n# 1 - T\n"
        )
    };
    let dir = repo(
        "record-header",
        &cfg,
        &[
            ("TASK-1-a.md", header("record:src/a.rs")),
            ("TASK-2-b.md", header("record:docs/adr/ADR-1-use-x.md")),
        ],
    );
    let (_, report) = run(&dir, &["check"]);
    let found = findings(&report, "embodiment.locator-exists");
    assert_eq!(found.len(), 1, "{report}");
    assert!(
        found[0]["file"].as_str().unwrap().contains("TASK-1"),
        "{report}"
    );
    assert!(
        found[0]["message"]
            .as_str()
            .unwrap()
            .contains("under `record:`, which is not a record of any declared type"),
        "{report}"
    );
}

// -------------------------- init proposes none of the rules that apply by declaration

const BY_DECLARATION: [&str; 5] = [
    "status.delivery-agreement",
    "status.done-items-shape",
    "status.waived-item-stale",
    "status.required-field-missing",
    "status.forbidden-field-present",
];

fn rules_written(dir: &Path) -> String {
    let text = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    text[text.find("\nrules:").expect("a rules table")..].to_string()
}

#[test]
fn init_proposes_none_of_the_rules_that_apply_by_declaration() {
    let adopt = common::TempDir::new("urzua-checklist-init-adopt");
    std::fs::create_dir_all(adopt.join("docs/adr")).unwrap();
    std::fs::write(
        adopt.join("docs/adr/ADR-0001-x.md"),
        "---\nStatus: Accepted\nDate: 2026-01-01\nDeciders: a\n---\n# 1 - X\n",
    )
    .unwrap();
    common::git(&adopt, &["init", "-q"]);
    common::git(&adopt, &["add", "-A"]);
    let out = common::urzua()
        .arg("init")
        .current_dir(&adopt)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let green = common::TempDir::new("urzua-checklist-init-green");
    common::git(&green, &["init", "-q"]);
    let out = common::urzua()
        .args(["init", "--types", "adr,rfc,spec,prd", "--dir", "docs"])
        .current_dir(&green)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    for dir in [&adopt, &green] {
        let rules = rules_written(dir);
        for rule in BY_DECLARATION {
            assert!(!rules.contains(rule), "{rule} is not proposed:\n{rules}");
        }
    }
}

#[test]
fn a_category_no_tier_declares_is_named_once_each_in_first_seen_order() {
    let message = done_cause(
        "- Built. — evidence: code:src/a.rs, spec:docs/adr/ADR-1-use-x.md, code:src/a.rs\n",
        &SMALL_CHECKLIST.replace("[Implemented, Verified]", "[Verified]"),
        "embodiment:
  drift: Drift detected
  tiers:
    - {state: Verified, evidence: test, claims_built: true}
  none: Not started
",
    );
    assert!(
        message.contains("declares `evidence: code`, `evidence: spec`:"),
        "{message}"
    );
}

#[test]
fn a_rule_named_under_rules_counts_toward_ok_even_when_it_read_no_record() {
    let dir = note_repo("named-reads-nothing", None);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{REQUIRE_ONLY}rules:\n  status.required-field-missing: error\n"),
    )
    .unwrap();
    let (code, report) = run(&dir, &["check"]);
    assert_eq!(report["status"], "ok", "{report}");
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["records_read_by_any_rule"], 0, "{report}");
}
