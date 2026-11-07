//! Tests of the acceptance coverage gate (SPEC-4, "Coverage gate").

#[path = "acceptance/coverage_gate.rs"]
mod coverage_gate;

use coverage_gate::{parse, violations, ALLOW_LIST, CEILING};
use std::collections::BTreeSet;

fn allow(rules: &[&str]) -> Vec<coverage_gate::Allowed> {
    rules
        .iter()
        .map(|r| coverage_gate::Allowed {
            rule: r.to_string(),
            reason: "r".to_string(),
        })
        .collect()
}

#[test]
fn the_committed_allow_list_parses_and_stays_within_the_ceiling() {
    let list = parse(ALLOW_LIST).unwrap();
    for a in &list {
        assert!(
            CEILING.contains(&a.rule.as_str()),
            "{} is not in the ceiling",
            a.rule
        );
        assert!(!a.reason.trim().is_empty(), "{} has no reason", a.rule);
    }
}

#[test]
fn every_rule_listed_or_detected_passes() {
    let all = ["a", "b", "c"];
    let detected: BTreeSet<&str> = ["a", "b"].into();
    assert!(violations(&all, &detected, &allow(&["c"]), &["c"]).is_empty());
}

#[test]
fn a_new_rule_without_a_failing_case_fails_the_gate() {
    let all = ["a", "b", "brand.new"];
    let detected: BTreeSet<&str> = ["a", "b"].into();
    let v = violations(&all, &detected, &allow(&[]), &[]);
    assert_eq!(v.len(), 1, "{v:?}");
    assert!(v[0].contains("brand.new"), "{v:?}");
}

#[test]
fn listing_the_new_rule_does_not_rescue_it() {
    let all = ["a", "brand.new"];
    let detected: BTreeSet<&str> = ["a"].into();
    let v = violations(&all, &detected, &allow(&["brand.new"]), &["old"]);
    assert!(
        v.iter()
            .any(|m| m.contains("brand.new") && m.contains("only shrinks")),
        "{v:?}"
    );
}

#[test]
fn an_entry_whose_rule_gained_a_case_or_left_the_table_is_stale() {
    let all = ["a", "b"];
    let detected: BTreeSet<&str> = ["a"].into();
    let v = violations(&all, &detected, &allow(&["a", "gone"]), &["a", "gone"]);
    assert!(v.iter().any(|m| m.contains("a is stale")), "{v:?}");
    assert!(
        v.iter()
            .any(|m| m.contains("gone") && m.contains("not in the rule table")),
        "{v:?}"
    );
}

#[test]
fn an_entry_without_a_reason_fails() {
    let all = ["a"];
    let detected: BTreeSet<&str> = BTreeSet::new();
    let mut list = allow(&["a"]);
    list[0].reason = "  ".to_string();
    let v = violations(&all, &detected, &list, &["a"]);
    assert!(v.iter().any(|m| m.contains("no reason")), "{v:?}");
}
