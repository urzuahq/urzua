//! The nine worked examples of `ADR-96` section 1, each a fixture, and the
//! grammar's other rules.

use super::*;

fn decl() -> ItemsDecl {
    yaml_serde::from_str(
        r#"
section: "Done means"
clause_separator: " — "
evidence_marker: "evidence:"
waiver_marker: "waived:"
partial_status: "InProgress"
waiver_categories:
  - {name: not-applicable}
  - {name: superseded-by-decision, cite: required}
  - {name: deferred-to, cite: required, expires: required}
  - {name: process-outcome}
"#,
    )
    .expect("the declaration parses")
}

fn parse(body: &str) -> DoneItems {
    parse_items(body, 10, 9, &decl())
}

fn issues(d: &DoneItems) -> Vec<String> {
    d.issues
        .iter()
        .map(|i| format!("item {:?}: {}", i.item, i.message))
        .collect()
}

#[test]
fn example_1_two_locators_the_comma_followed_by_a_category_separates() {
    let d = parse("- A declared alias satisfies the section requirement and is reported as the alias used. — evidence: code:rust/crates/urzua-core/src/rules/section.rs#section_required, test:rust/crates/urzua-cli/tests/section_rules_integration.rs#an_alias_satisfies_the_requirement\n");
    assert!(d.issues.is_empty(), "{:?}", issues(&d));
    assert_eq!(d.items.len(), 1);
    let item = &d.items[0];
    assert_eq!(item.n, 1);
    assert_eq!(
        item.text,
        "A declared alias satisfies the section requirement and is reported as the alias used."
    );
    let clause = item.evidence.as_ref().expect("an evidence clause");
    assert!(clause.valid);
    let entries = crate::locator::realized_by_entries(&clause.text);
    assert_eq!(entries.len(), 2, "{entries:?}");
    assert_eq!(entries[0].0, "code");
    assert_eq!(entries[1].0, "test");
}

#[test]
fn example_2_a_comma_inside_a_path_stays_in_the_path() {
    let d = parse("- Config is read from a directory whose name has a comma. — evidence: code:rust/tools/a,b/src/lib.rs, test:rust/tools/a,b/tests/t.rs\n");
    assert!(d.issues.is_empty(), "{:?}", issues(&d));
    let clause = d.items[0].evidence.as_ref().unwrap();
    let entries = crate::locator::realized_by_entries(&clause.text);
    assert_eq!(entries.len(), 2, "{entries:?}");
    assert_eq!(entries[0].1.trim(), "rust/tools/a,b/src/lib.rs");
    assert_eq!(entries[1].1.trim(), "rust/tools/a,b/tests/t.rs");
}

#[test]
fn example_3_a_marker_word_without_the_separator_is_text() {
    let d = parse("- The report prints evidence: per item, with no separator before the word.\n");
    assert!(d.issues.is_empty(), "{:?}", issues(&d));
    assert_eq!(d.items.len(), 1);
    assert!(d.items[0].evidence.is_none());
    assert!(d.items[0].waiver.is_none());
    assert_eq!(
        d.items[0].text,
        "The report prints evidence: per item, with no separator before the word."
    );
}

#[test]
fn example_4_sub_bullets_are_the_items_text_and_the_evidence_is_on_the_final_line() {
    let d = parse(
        "- Every rule that reads a section reads it through one function.\n  - `section.required` calls it\n  - `section.order` calls it\n  - the harness text calls it\n\n  evidence: code:rust/crates/urzua-core/src/rules/section.rs#sections_of, test:rust/crates/urzua-core/src/rules/section.rs#every_reader_shares_one_function\n",
    );
    assert!(d.issues.is_empty(), "{:?}", issues(&d));
    assert_eq!(d.items.len(), 1, "sub-bullets are never items");
    let clause = d.items[0].evidence.as_ref().expect("final-line clause");
    assert!(clause.valid);
    assert_eq!(clause.line, 10 + 5);
    assert!(clause.text.starts_with("code:rust/crates/urzua-core"));
}

#[test]
fn example_5_an_ordered_item_and_a_marker_inside_a_reason() {
    let d = parse("1. The release notes name the change. — waived: process-outcome -- the notes are written when the release PR is opened, see evidence: CI and #512\n");
    assert!(d.issues.is_empty(), "{:?}", issues(&d));
    let item = &d.items[0];
    assert!(item.evidence.is_none());
    let waiver = item.waiver.as_ref().expect("a waiver");
    assert_eq!(waiver.category, "process-outcome");
    assert_eq!(
        waiver.reason,
        "the notes are written when the release PR is opened, see evidence: CI and #512"
    );
}

#[test]
fn example_6_evidence_and_waiver_together() {
    let d = parse("- A second platform builds. — evidence: test:rust/crates/urzua-cli/tests/platform_windows.rs#builds — waived: deferred-to (MILE-230, 2027-01-31) -- the runner is not available until MILE-230 lands citation-fixture\n");
    assert!(d.issues.is_empty(), "{:?}", issues(&d));
    let item = &d.items[0];
    assert_eq!(
        item.evidence.as_ref().unwrap().text,
        "test:rust/crates/urzua-cli/tests/platform_windows.rs#builds"
    );
    let w = item.waiver.as_ref().unwrap();
    assert_eq!(w.category, "deferred-to");
    assert_eq!(w.cite.as_deref(), Some("MILE-230"));
    assert_eq!(w.date.as_deref(), Some("2027-01-31"));
    assert!(w.reason.starts_with("the runner is not available"));
}

#[test]
fn example_7_backticks_before_the_first_category_are_a_shape_issue() {
    let d = parse("- The reader accepts `code:` locators. — evidence: `code:rust/crates/urzua-core/src/locator.rs`\n");
    assert_eq!(d.issues.len(), 1, "{:?}", issues(&d));
    assert_eq!(d.issues[0].item, Some(1));
    assert!(
        d.issues[0].message.contains("belongs to no locator entry"),
        "{:?}",
        issues(&d)
    );
    assert!(!d.items[0].evidence.as_ref().unwrap().valid);
}

#[test]
fn example_8_whitespace_in_a_locator_names_the_stray_text() {
    let d = parse("- The reader is fast. — evidence: test:rust/crates/urzua-core/benches/read.rs because it measures it\n");
    assert_eq!(d.issues.len(), 1, "{:?}", issues(&d));
    assert!(
        d.issues[0].message.contains("because it measures it"),
        "{:?}",
        issues(&d)
    );
    assert!(!d.items[0].evidence.as_ref().unwrap().valid);
}

#[test]
fn example_9_a_leading_checkbox_is_a_shape_issue() {
    let d = parse(
        "- [x] The doc names the command. — evidence: spec:docs/specs/SPEC-2-urzua-check.md\n",
    );
    assert_eq!(d.issues.len(), 1, "{:?}", issues(&d));
    assert!(d.issues[0].message.contains("checkbox"), "{:?}", issues(&d));
}

#[test]
fn the_nine_examples_in_one_section_are_numbered_by_position() {
    let body = "- one — evidence: code:a.rs\n- two — evidence: code:b.rs\n  - sub\n3. three\n";
    let d = parse(body);
    let ns: Vec<usize> = d.items.iter().map(|i| i.n).collect();
    assert_eq!(ns, vec![1, 2, 3]);
    assert_eq!(d.items[2].line, 13);
}

#[test]
fn a_prose_only_section_is_one_shape_issue() {
    let d = parse("One paragraph of semicolon-joined clauses; each should be an item.\n");
    assert!(d.items.is_empty());
    assert_eq!(d.issues.len(), 1);
    assert_eq!(d.issues[0].line, 9);
    assert!(d.issues[0].message.contains("prose only"));
}

#[test]
fn clauses_on_the_first_line_and_on_final_lines_at_once_are_a_shape_issue() {
    let d = parse("- text — evidence: code:a.rs\n  evidence: code:b.rs\n");
    assert_eq!(d.issues.len(), 1, "{:?}", issues(&d));
    assert!(d.issues[0].message.contains("at once"));
}

#[test]
fn a_marker_in_a_middle_continuation_line_is_plain_text() {
    let d = parse("- text\n  evidence: code:a.rs mentioned in passing\n  and more words\n");
    assert!(d.issues.is_empty(), "{:?}", issues(&d));
    assert!(d.items[0].evidence.is_none());
}

#[test]
fn a_final_waiver_before_the_evidence_is_a_shape_issue() {
    let d = parse("- text\n  waived: not-applicable -- gone\n  evidence: code:a.rs\n");
    assert_eq!(d.issues.len(), 1, "{:?}", issues(&d));
    assert!(d.issues[0].message.contains("evidence clause comes first"));
}

#[test]
fn a_first_line_waiver_before_the_evidence_is_a_shape_issue_and_the_evidence_is_kept() {
    let d = parse(
        "- X works. — waived: deferred-to (MILE-1, 2099-01-01) -- later — evidence: code:a.rs\n",
    );
    assert_eq!(d.issues.len(), 1, "{:?}", issues(&d));
    assert!(d.issues[0].message.contains("evidence clause comes first"));
    let item = &d.items[0];
    assert_eq!(item.evidence.as_ref().unwrap().text, "code:a.rs");
    assert_eq!(item.waiver.as_ref().unwrap().reason, "later");
}

#[test]
fn a_clause_line_that_belongs_to_no_item_is_a_shape_issue() {
    let d = parse("- item\nevidence: code:a.rs\n");
    assert!(
        issues(&d).iter().any(|m| m.contains("belongs to no item")),
        "{:?}",
        issues(&d)
    );
}

#[test]
fn a_marker_indented_less_than_the_item_indent_is_a_shape_issue() {
    let d = parse("  - first\n- second\n");
    assert!(
        issues(&d).iter().any(|m| m.contains("indented less")),
        "{:?}",
        issues(&d)
    );
}

#[test]
fn waiver_shape_problems_are_each_a_shape_issue() {
    let cases = [
        (
            "- t — waived: made-up -- r\n",
            "not a declared waiver category",
        ),
        ("- t — waived: not-applicable -- \n", "no ` -- <reason>`"),
        (
            "- t — waived: superseded-by-decision -- r\n",
            "requires a cite",
        ),
        (
            "- t — waived: superseded-by-decision (PR 5) -- r\n",
            "not a record id",
        ),
        (
            "- t — waived: deferred-to (MILE-9) -- r\n",
            "requires an expiry date",
        ),
        (
            "- t — waived: deferred-to (MILE-9, 2027-02-30) -- r\n",
            "not a real",
        ),
        (
            "- t — waived: not-applicable (MILE-9, 2027-01-01) -- r\n",
            "takes no date",
        ),
        ("- t — waived: process-outcome\n", "no ` -- <reason>`"),
    ];
    for (body, want) in cases {
        let d = parse(body);
        assert!(
            issues(&d).iter().any(|m| m.contains(want)),
            "{body:?} wanted {want:?}, got {:?}",
            issues(&d)
        );
        assert!(d.items[0].waiver.is_none(), "{body:?}");
    }
}

#[test]
fn a_fenced_block_is_skipped() {
    let d = parse("```\n- not an item — evidence: code:a.rs\n```\n- real — evidence: code:b.rs\n");
    assert_eq!(d.items.len(), 1);
    assert_eq!(d.items[0].text, "real");
}

#[test]
fn locator_text_joins_every_valid_clause() {
    let d = parse("- a — evidence: code:a.rs\n- b\n- c — evidence: test:c.rs, spec:docs/x.md\n");
    assert_eq!(
        d.locator_text().as_deref(),
        Some("code:a.rs, test:c.rs, spec:docs/x.md")
    );
}

#[test]
fn valid_evidence_lines_skip_a_clause_that_does_not_parse() {
    let d = parse("- a — evidence: `code:a.rs`\n- b — evidence: code:b.rs\n");
    assert_eq!(d.evidence_lines().len(), 2);
    assert_eq!(d.valid_evidence_lines().len(), 1);
    assert_eq!(d.valid_evidence_lines()[0], d.evidence_lines()[1]);
}

fn decl_with_pending() -> ItemsDecl {
    let mut d = decl();
    d.pending = Some(vec![
        crate::status_model::PendingEntry::Value("TODO".to_string()),
        crate::status_model::PendingEntry::Pattern(crate::status_model::PendingPattern {
            pattern: "^Replace this item".to_string(),
        }),
    ]);
    d
}

#[test]
fn a_pending_item_is_a_placeholder_and_a_shape_issue_naming_its_line() {
    let d = parse_items(
        "- Replace this item with one criterion.\n- todo\n- A real criterion. — evidence: code:rust/a.rs\n",
        10,
        9,
        &decl_with_pending(),
    );
    assert!(d.items[0].placeholder && d.items[1].placeholder);
    assert!(!d.items[2].placeholder);
    let lines: Vec<usize> = d.issues.iter().map(|i| i.line).collect();
    assert_eq!(lines, vec![10, 11], "{:?}", issues(&d));
    assert!(d.issues[0].message.contains("unfilled placeholder"));
}

#[test]
fn a_placeholder_item_carries_no_evidence_and_no_waiver() {
    let d = parse_items(
        "- TODO — evidence: code:rust/a.rs\n- Replace this item — waived: not-applicable -- nothing to do\n",
        10,
        9,
        &decl_with_pending(),
    );
    assert!(d.items.iter().all(|i| i.placeholder));
    assert!(d
        .items
        .iter()
        .all(|i| i.evidence.is_none() && i.waiver.is_none()));
    assert_eq!(d.locator_text(), None);
}

#[test]
fn an_undeclared_pending_changes_nothing() {
    let body = "- Replace this item with one criterion.\n- TODO\n";
    let d = parse(body);
    assert!(d.items.iter().all(|i| !i.placeholder));
    assert!(d.issues.is_empty(), "{:?}", issues(&d));
}
