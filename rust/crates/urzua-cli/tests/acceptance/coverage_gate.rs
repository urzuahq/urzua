//! The acceptance coverage gate (SPEC-4, "Coverage gate"): every rule in the
//! rule table has a failing acceptance case unless it is on the committed
//! allow-list, and the allow-list only shrinks.
//!
//! Shared by the `acceptance` runner, which enforces it on an unfiltered run,
//! and by `acceptance_coverage_gate.rs`, which tests it.

use serde::Deserialize;
use std::collections::BTreeSet;

/// One allow-list entry: a rule with no failing case, and why.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Allowed {
    pub rule: String,
    pub reason: String,
}

/// The most the allow-list may ever hold. An entry outside this set is new, and
/// a new entry is refused: a rule the table gains must come with a failing
/// case. Delete an id here when its entry is deleted from the allow-list.
pub const CEILING: &[&str] = &[
    "claim.status-agreement",
    "embodiment.back-pointer-decision-accepted",
    "config.schema-version-outdated",
    "index.freshness",
    "identity.open-pr-collision",
];

/// The committed allow-list text.
pub const ALLOW_LIST: &str = include_str!("coverage-allowlist.yaml");

pub fn parse(text: &str) -> Result<Vec<Allowed>, String> {
    yaml_serde::from_str(text).map_err(|e| format!("coverage-allowlist.yaml: {e}"))
}

/// Every reason the gate fails, empty when it passes. `detected` holds the
/// rules that have a failing case.
pub fn violations(
    all_rules: &[&str],
    detected: &BTreeSet<&str>,
    allow: &[Allowed],
    ceiling: &[&str],
) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for a in allow {
        if !seen.insert(a.rule.as_str()) {
            out.push(format!("allow-list names {} twice", a.rule));
        }
        if a.reason.trim().is_empty() {
            out.push(format!("allow-list entry {} has no reason", a.rule));
        }
        if !ceiling.contains(&a.rule.as_str()) {
            out.push(format!(
                "allow-list entry {} is new: the list only shrinks, so write a failing case for the rule instead",
                a.rule
            ));
        }
        if !all_rules.contains(&a.rule.as_str()) {
            out.push(format!(
                "allow-list entry {} names a rule that is not in the rule table: remove the entry",
                a.rule
            ));
        } else if detected.contains(a.rule.as_str()) {
            out.push(format!(
                "allow-list entry {} is stale: the rule has a failing case now, remove the entry",
                a.rule
            ));
        }
    }
    for rule in all_rules {
        if !detected.contains(rule) && !seen.contains(rule) {
            out.push(format!(
                "rule {rule} has no failing acceptance case and is not on the allow-list: add a case that expects its finding"
            ));
        }
    }
    out
}
