//! Tests of the acceptance bug-coverage gate (SPEC-4, "Bug coverage gate"), and
//! the gate itself over this repository. No corpus is run: it reads the bug
//! records, the case files and the committed lists, so it is fast and exact.

#[path = "acceptance/bug_gate.rs"]
mod bug_gate;

use bug_gate::{
    header_status, load, parse_ceilings, parse_gap_file, parse_ledger, violations, Facts, Ledgered,
    Tag,
};
use std::path::Path;

fn facts(fixed: &[&str]) -> Facts {
    Facts {
        statuses: fixed
            .iter()
            .map(|b| (b.to_string(), "Fixed".to_string()))
            .collect(),
        ..Facts::default()
    }
}

fn tag(f: &mut Facts, case: &str, bug: &str) {
    f.tags.push(Tag {
        case: case.to_string(),
        bug: bug.to_string(),
    });
}

fn gap(f: &mut Facts, lane: &str, bug: &str, ceiling: bool) {
    f.gaps.push((lane.to_string(), bug.to_string()));
    if ceiling {
        f.ceiling.push((bug.to_string(), lane.to_string()));
    }
}

fn ledger(f: &mut Facts, bug: &str, reason: &str) {
    f.ledger.push(Ledgered {
        bug: bug.to_string(),
        reason: reason.to_string(),
    });
}

fn has(v: &[String], needles: &[&str]) -> bool {
    v.iter().any(|m| needles.iter().all(|n| m.contains(n)))
}

#[test]
fn the_repository_passes_the_gate() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let facts = load(&root).unwrap();
    let v = violations(&facts);
    assert!(v.is_empty(), "bug coverage gate:\n  {}", v.join("\n  "));
}

#[test]
fn a_bug_that_is_tagged_gapped_or_ledgered_passes() {
    let mut f = facts(&["BUG-1", "BUG-2", "BUG-3"]);
    tag(&mut f, "c", "BUG-1");
    gap(&mut f, "a", "BUG-2", true);
    ledger(&mut f, "BUG-3", "a release workflow");
    assert!(violations(&f).is_empty(), "{:?}", violations(&f));
}

#[test]
fn a_fixed_bug_with_no_case_gap_or_ledger_entry_fails() {
    let f = facts(&["BUG-1", "BUG-2"]);
    let v = violations(&f);
    assert_eq!(v.len(), 2, "{v:?}");
    assert!(has(&v, &["BUG-1", "no case tags it"]), "{v:?}");
}

#[test]
fn a_case_tagging_an_unknown_bug_fails() {
    let mut f = facts(&["BUG-1"]);
    tag(&mut f, "c", "BUG-1");
    tag(&mut f, "c2", "BUG-999");
    let v = violations(&f);
    assert!(has(&v, &["c2", "BUG-999", "not a bug record"]), "{v:?}");
}

#[test]
fn a_case_tagging_a_bug_that_is_not_fixed_fails() {
    let mut f = facts(&["BUG-1"]);
    f.statuses.insert("BUG-2".to_string(), "Open".to_string());
    tag(&mut f, "c", "BUG-1");
    tag(&mut f, "c2", "BUG-2");
    let v = violations(&f);
    assert!(has(&v, &["c2", "BUG-2", "is Open, not Fixed"]), "{v:?}");
}

#[test]
fn a_gap_entry_outside_the_ceiling_is_refused() {
    let mut f = facts(&["BUG-1", "BUG-2"]);
    gap(&mut f, "a", "BUG-1", true);
    gap(&mut f, "b", "BUG-2", false);
    let v = violations(&f);
    assert_eq!(v.len(), 1, "{v:?}");
    assert!(has(&v, &["BUG-2", "is new", "only shrink"]), "{v:?}");
}

#[test]
fn moving_a_gap_to_another_lane_is_refused() {
    let mut f = facts(&["BUG-1"]);
    f.gaps.push(("b".to_string(), "BUG-1".to_string()));
    f.ceiling.push(("BUG-1".to_string(), "a".to_string()));
    assert!(has(&violations(&f), &["BUG-1", "lane b", "lane a"]));
}

#[test]
fn a_gap_a_case_now_tags_is_stale() {
    let mut f = facts(&["BUG-1"]);
    gap(&mut f, "a", "BUG-1", true);
    tag(&mut f, "c", "BUG-1");
    let v = violations(&f);
    assert!(has(&v, &["BUG-1", "stale"]), "{v:?}");
}

#[test]
fn a_ceiling_line_with_no_gap_entry_must_be_deleted() {
    let mut f = facts(&["BUG-1"]);
    tag(&mut f, "c", "BUG-1");
    f.ceiling.push(("BUG-1".to_string(), "a".to_string()));
    assert!(has(
        &violations(&f),
        &["BUG-1", "in no gap file", "only shrinks"]
    ));
}

#[test]
fn a_bug_in_two_places_fails() {
    let mut f = facts(&["BUG-1", "BUG-2"]);
    gap(&mut f, "a", "BUG-1", true);
    gap(&mut f, "b", "BUG-1", true);
    gap(&mut f, "c", "BUG-2", true);
    ledger(&mut f, "BUG-2", "a refactor");
    let v = violations(&f);
    assert!(has(&v, &["gap files", "BUG-1", "twice"]), "{v:?}");
    assert!(has(&v, &["BUG-2", "ledger and a gap file"]), "{v:?}");
}

#[test]
fn a_ledger_entry_needs_a_one_line_reason_and_a_fixed_bug() {
    let mut f = facts(&["BUG-1", "BUG-2"]);
    f.statuses
        .insert("BUG-3".to_string(), "WontFix".to_string());
    ledger(&mut f, "BUG-1", "  ");
    ledger(&mut f, "BUG-2", "first\nsecond");
    ledger(&mut f, "BUG-3", "release");
    let v = violations(&f);
    assert!(has(&v, &["BUG-1", "one-line reason"]), "{v:?}");
    assert!(has(&v, &["BUG-2", "one-line reason"]), "{v:?}");
    assert!(has(&v, &["BUG-3", "not Fixed"]), "{v:?}");
}

#[test]
fn a_ledger_entry_a_case_now_tags_is_stale() {
    let mut f = facts(&["BUG-1"]);
    ledger(&mut f, "BUG-1", "ci");
    tag(&mut f, "c", "BUG-1");
    assert!(has(&violations(&f), &["BUG-1", "stale"]));
}

#[test]
fn the_list_formats_parse_and_malformed_ones_are_refused() {
    assert_eq!(parse_gap_file("a", "- BUG-1\n- BUG-2\n").unwrap().len(), 2);
    assert!(parse_gap_file("a", "- {bug: BUG-1}\n").is_err());
    assert_eq!(
        parse_ledger("- {bug: BUG-1, reason: ci}\n").unwrap().len(),
        1
    );
    assert!(parse_ledger("- {bug: BUG-1, reason: ci, extra: 1}\n").is_err());
    assert_eq!(parse_ceilings("# c\n\nBUG-1\ta\n").unwrap().len(), 1);
    assert!(parse_ceilings("BUG-1 a\n").is_err());
}

#[test]
fn a_status_is_read_from_the_frontmatter_only() {
    assert_eq!(
        header_status("---\nStatus: Fixed\n---\nStatus: Open\n").as_deref(),
        Some("Fixed")
    );
    assert_eq!(
        header_status("---\nStable-Id: x\n---\nStatus: Open\n"),
        None
    );
    assert_eq!(header_status("# no header\nStatus: Fixed\n"), None);
}
