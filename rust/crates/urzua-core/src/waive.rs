//! `urzua waive --from-report` (ADR-91): which findings become a draft waiver,
//! how they group, and what the draft's `Reason` is. Pure: the command runs
//! `check`, reads the records and writes the files; this module judges.
//!
//! One draft per rule and file. A draft suppresses nothing until a person
//! writes the reason and the date, so no finding is hidden by the tool.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::config::FailOn;
use crate::report::{Finding, FindingSeverity};
use crate::status_model::PendingEntry;
use crate::waiver::{Suppression, SuppressionType};
use crate::FieldState;

/// The field a draft's pending value goes in, and the one the type must require.
pub const REASON_FIELD: &str = crate::new_record::REASON_FIELD;
/// The field a draft leaves unwritten, so it cannot be permanent by accident.
pub const EXPIRES_FIELD: &str = crate::new_record::EXPIRES_FIELD;

/// Where the draft's pending `Reason` came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PendingSource {
    /// The type's first whole-value `statuses.pending` entry.
    Declared,
    /// The compiled-in `Pending`, the value the engine reads as pending for a
    /// type that declares no vocabulary.
    Default,
}

/// The value a draft's `Reason` holds, and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingChoice {
    pub value: String,
    pub source: PendingSource,
}

/// The value fallback when the type declares no `pending` vocabulary.
const DEFAULT_PENDING: &str = "Pending";

fn whole_value(entry: &PendingEntry) -> Option<String> {
    match entry {
        PendingEntry::Value(v) => Some(v.trim().to_string()),
        PendingEntry::Pattern(_) => None,
    }
}

fn is_whole_value(entry: &PendingEntry) -> bool {
    whole_value(entry).is_some()
}

fn default_choice() -> PendingChoice {
    PendingChoice {
        value: DEFAULT_PENDING.to_string(),
        source: PendingSource::Default,
    }
}

/// What the draft's `Reason` is, or why the command must not run (exit 2,
/// nothing written). Each refusal names its fix. `ADR-91` points 7 and 9: a
/// pending value that suppressed, or a waiver that could become permanent by
/// accident, is the baseline file the decision rejects.
pub fn choose_pending(st: &SuppressionType) -> Result<PendingChoice, String> {
    if !st.effective_requires.iter().any(|f| f == REASON_FIELD) {
        return Err(format!(
            "the {} type does not list {REASON_FIELD} in `effective_requires`, so a draft with a pending {REASON_FIELD} would suppress its finding; add {REASON_FIELD} to `effective_requires` and run again",
            st.name
        ));
    }
    let choice = match st.declared.as_ref().and_then(|d| d.pending.as_ref()) {
        // A non-empty list with no whole value: nothing to write.
        Some(entries) if !entries.is_empty() && !entries.iter().any(is_whole_value) => {
            return Err(format!(
                "the {} type's `statuses.pending` holds only patterns, so there is no whole value to write as the {REASON_FIELD}; add a whole-value entry (for example `pending: [Pending, {{pattern: ...}}]`)",
                st.name
            ))
        }
        Some(entries) => match entries.iter().find_map(whole_value) {
            Some(value) => PendingChoice {
                value,
                source: PendingSource::Declared,
            },
            // `pending: []` declares nothing pending; the default vocabulary
            // still reads a whole `Pending` as pending (`field_state`).
            None => default_choice(),
        },
        None => default_choice(),
    };
    ensure_not_present(st, &choice.value)?;
    if st.permanent.is_none() && !st.effective_requires.iter().any(|f| f == EXPIRES_FIELD) {
        return Err(format!(
            "the {} type neither declares `permanent` nor lists {EXPIRES_FIELD} in `effective_requires`, so a draft given only a {REASON_FIELD} would be a permanent waiver; declare `permanent` (ADR-86) or add {EXPIRES_FIELD} to `effective_requires`, then run again",
            st.name
        ));
    }
    Ok(choice)
}

/// Refuses a value the type reads as a real `Reason`: it would suppress.
fn ensure_not_present(st: &SuppressionType, value: &str) -> Result<(), String> {
    if st.field_state(REASON_FIELD, Some(value)) == FieldState::Present {
        return Err(format!(
            "the {} type reads '{value}' as a real {REASON_FIELD}, not a pending one, so a draft holding it would suppress its finding; declare it under `statuses.pending` or choose a pending value",
            st.name
        ));
    }
    Ok(())
}

/// Why a finding is not drafted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkipReason {
    /// A finding of `waiver.matches-nothing`: a waiver for a waiver would hide
    /// the report that says a waiver does nothing.
    MatchesNothing,
    /// The finding is tied to no file, so no scope names it.
    NoFile,
    /// A waiver with the same rule and scope exists (effective or not).
    AlreadyWaived,
    /// A waiver with the same rule and scope exists and its `Expires` has
    /// passed; it is renewed, not drafted again.
    Expired,
    /// The rule declares the finding's variant unwaivable, so a waiver can
    /// never cover it.
    Unwaivable,
    /// Writing the drafts resolves the finding (`ADR-91`, amendment of 2026-11-08),
    /// judged again on the drafts that remain.
    ResolvedByDrafts,
    /// The finding disappears only if its own draft exists and nothing else is
    /// drafted, so no draft is written and the finding remains.
    ResolvedByOwnDraft,
    /// The finding is in a waiver record, a draft's own pending `Reason` for
    /// one: drafting waivers for waivers would never end.
    WaiverRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Skipped {
    pub rule: String,
    pub file: Option<String>,
    pub reason: SkipReason,
    /// The existing waiver record, for `already-waived` and `expired`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiver: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FilterKind {
    Rule,
    Path,
}

/// A `--rule` or `--path` that matched no finding.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct UnmatchedFilter {
    pub kind: FilterKind,
    pub value: String,
}

/// One draft's worth of findings: one rule in one file.
#[derive(Debug, Clone)]
pub struct Group {
    pub rule: String,
    /// The finding's path exactly as `Suppression::covers` compares it.
    pub scope: String,
    pub findings: Vec<Finding>,
}

impl Group {
    /// The findings as the report and the draft's body show them, verbatim:
    /// `file:line message`, or `file message` for a finding with no line.
    pub fn lines(&self) -> Vec<String> {
        self.findings.iter().map(finding_line).collect()
    }
}

fn finding_line(f: &Finding) -> String {
    let file = f.file.to_string_lossy();
    match f.line {
        Some(line) => format!("{file}:{line} {}", f.message),
        None => format!("{file} {}", f.message),
    }
}

/// The `--rule` and `--path` filters, `--path` already relative to the
/// repository root.
#[derive(Debug, Clone, Default)]
pub struct Filters {
    pub rules: Vec<String>,
    pub paths: Vec<PathBuf>,
}

/// What the report supports drafting, what it passes over and which filters
/// found nothing.
#[derive(Debug, Default)]
pub struct Plan {
    pub groups: Vec<Group>,
    pub skipped: Vec<Skipped>,
    pub unmatched: Vec<UnmatchedFilter>,
}

fn at_or_above(severity: FindingSeverity, fail_on: FailOn) -> bool {
    match severity {
        FindingSeverity::Error => true,
        FindingSeverity::Warning => fail_on == FailOn::Warning,
        FindingSeverity::Info => false,
    }
}

fn path_matches(file: &std::path::Path, filter: &std::path::Path) -> bool {
    file.starts_with(filter)
}

/// Judges the report. A finding is a candidate when it is unwaived and at or
/// above `fail_on`; a filter narrows the candidates, and a filter that matches
/// none is listed. Info findings are never listed (`ADR-91` point 10).
pub fn plan(
    findings: &[Finding],
    fail_on: FailOn,
    (waivers, waiver_records): (&[Suppression], &HashSet<PathBuf>),
    today: &str,
    filters: &Filters,
) -> Plan {
    let pool: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.waived.is_none() && at_or_above(f.severity, fail_on))
        .collect();
    let mut out = Plan {
        unmatched: unmatched_filters(&pool, filters),
        ..Plan::default()
    };
    let mut groups: Vec<Group> = Vec::new();
    let mut skipped: Vec<Skipped> = Vec::new();
    for f in pool.iter().filter(|f| in_filters(f, filters)) {
        match skip_reason(f, (waivers, waiver_records), today) {
            Some((reason, waiver)) => {
                let entry = Skipped {
                    rule: f.rule.clone(),
                    file: (!f.file.as_os_str().is_empty())
                        .then(|| f.file.to_string_lossy().into_owned()),
                    reason,
                    waiver,
                };
                if !skipped.contains(&entry) {
                    skipped.push(entry);
                }
            }
            None => {
                let scope = f.file.to_string_lossy().into_owned();
                match groups
                    .iter_mut()
                    .find(|g| g.rule == f.rule && g.scope == scope)
                {
                    Some(g) => g.findings.push((*f).clone()),
                    None => groups.push(Group {
                        rule: f.rule.clone(),
                        scope,
                        findings: vec![(*f).clone()],
                    }),
                }
            }
        }
    }
    groups.sort_by(|a, b| (&a.scope, &a.rule).cmp(&(&b.scope, &b.rule)));
    skipped.sort_by(|a, b| (&a.file, &a.rule).cmp(&(&b.file, &b.rule)));
    out.groups = groups;
    out.skipped = skipped;
    out
}

fn in_filters(f: &Finding, filters: &Filters) -> bool {
    (filters.rules.is_empty() || filters.rules.contains(&f.rule))
        && (filters.paths.is_empty() || filters.paths.iter().any(|p| path_matches(&f.file, p)))
}

fn unmatched_filters(pool: &[&Finding], filters: &Filters) -> Vec<UnmatchedFilter> {
    let rules = filters
        .rules
        .iter()
        .filter(|r| !pool.iter().any(|f| &f.rule == *r))
        .map(|r| UnmatchedFilter {
            kind: FilterKind::Rule,
            value: r.clone(),
        });
    let paths = filters
        .paths
        .iter()
        .filter(|p| !pool.iter().any(|f| path_matches(&f.file, p)))
        .map(|p| UnmatchedFilter {
            kind: FilterKind::Path,
            value: p.to_string_lossy().into_owned(),
        });
    rules.chain(paths).collect()
}

/// Why `f` is not drafted, with the existing waiver when one is named.
fn skip_reason(
    f: &Finding,
    (waivers, waiver_records): (&[Suppression], &HashSet<PathBuf>),
    today: &str,
) -> Option<(SkipReason, Option<String>)> {
    if f.rule == crate::rules::RULE_WAIVER_MATCHES_NOTHING {
        return Some((SkipReason::MatchesNothing, None));
    }
    if f.file.as_os_str().is_empty() {
        return Some((SkipReason::NoFile, None));
    }
    if waiver_records.contains(&f.file) {
        return Some((SkipReason::WaiverRecord, None));
    }
    let scope = f.file.to_string_lossy();
    if let Some(w) = waivers
        .iter()
        .find(|w| w.rule == f.rule && w.scope == scope)
    {
        let reason = if w.is_active(today) {
            SkipReason::AlreadyWaived
        } else {
            SkipReason::Expired
        };
        return Some((reason, Some(w.id.clone())));
    }
    if !crate::rules::finding_is_waivable(f) {
        return Some((SkipReason::Unwaivable, None));
    }
    None
}

/// What `drop_resolved` made of the groups.
#[derive(Debug)]
pub struct Dropped {
    /// The groups with only the findings that `after` still holds.
    pub kept: Vec<Group>,
    /// The findings `after` no longer holds, grouped as they were.
    pub resolved: Vec<Group>,
}

/// Splits `groups` by whether `after` (the report on the tree with drafts in
/// memory) still holds each finding. A finding the drafts resolve is not
/// drafted (`ADR-91`, amendment of 2026-11-08). Matching is by rule, file,
/// line and message, each `after` finding used once.
pub fn drop_resolved(groups: Vec<Group>, after: &[Finding]) -> Dropped {
    type Key = (String, PathBuf, Option<usize>, String);
    let key = |f: &Finding| -> Key { (f.rule.clone(), f.file.clone(), f.line, f.message.clone()) };
    let mut remaining: HashMap<Key, usize> = HashMap::new();
    for f in after {
        *remaining.entry(key(f)).or_default() += 1;
    }
    let mut out = Dropped {
        kept: Vec::new(),
        resolved: Vec::new(),
    };
    for g in groups {
        let (present, gone): (Vec<Finding>, Vec<Finding>) =
            g.findings
                .into_iter()
                .partition(|f| match remaining.get_mut(&key(f)) {
                    Some(n) if *n > 0 => {
                        *n -= 1;
                        true
                    }
                    _ => false,
                });
        let part = |findings: Vec<Finding>| Group {
            rule: g.rule.clone(),
            scope: g.scope.clone(),
            findings,
        };
        if !gone.is_empty() {
            out.resolved.push(part(gone));
        }
        if !present.is_empty() {
            out.kept.push(part(present));
        }
    }
    out
}

/// One `skipped` entry per group, with `reason`, never dropped in silence.
pub fn skipped_groups(groups: &[Group], reason: SkipReason) -> Vec<Skipped> {
    groups
        .iter()
        .map(|g| Skipped {
            rule: g.rule.clone(),
            file: Some(g.scope.clone()),
            reason,
            waiver: None,
        })
        .collect()
}

/// `kept` with `more` folded in: findings join the group of the same rule and
/// file, and a new group takes its place in file then rule order.
pub fn merge_groups(mut kept: Vec<Group>, more: Vec<Group>) -> Vec<Group> {
    for g in more {
        match kept
            .iter_mut()
            .find(|k| k.rule == g.rule && k.scope == g.scope)
        {
            Some(k) => k.findings.extend(g.findings),
            None => kept.push(g),
        }
    }
    kept.sort_by(|a, b| (&a.scope, &a.rule).cmp(&(&b.scope, &b.rule)));
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::ActionSource;
    use crate::status_model::{PendingPattern, StatusDecl};

    fn finding(rule: &str, severity: FindingSeverity, file: &str, line: Option<usize>) -> Finding {
        let mut f = Finding::new(
            rule,
            severity,
            file,
            ActionSource::new("v"),
            format!("{rule} says so"),
        );
        f.line = line;
        f
    }

    fn error(rule: &str, file: &str) -> Finding {
        finding(rule, FindingSeverity::Error, file, None)
    }

    fn suppression(rule: &str, scope: &str, expires: Option<&str>) -> Suppression {
        Suppression {
            id: "docs/waiver/WAIVER-1-x.md".into(),
            rule: rule.into(),
            scope: scope.into(),
            expires: expires.map(str::to_string),
            unmet: Vec::new(),
            unmet_states: Vec::new(),
            permanence: crate::waiver::Permanence::Ungoverned,
        }
    }

    fn plan_of(findings: &[Finding], waivers: &[Suppression], filters: &Filters) -> Plan {
        plan(
            findings,
            FailOn::Warning,
            (waivers, &HashSet::new()),
            "2026-11-13",
            filters,
        )
    }

    fn st_with(pending: Option<Vec<PendingEntry>>) -> SuppressionType {
        let mut st = SuppressionType::named("waiver");
        st.effective_requires = vec!["Reason".into(), "Expires".into()];
        if pending.is_some() {
            st.declared = Some(StatusDecl {
                pending,
                ..serde_yaml_default()
            });
        }
        st
    }

    fn serde_yaml_default() -> StatusDecl {
        yaml_serde::from_str("{}").unwrap()
    }

    #[test]
    fn findings_group_by_rule_and_file_in_a_fixed_order() {
        let findings = [
            error("b.rule", "docs/z.md"),
            error("a.rule", "docs/z.md"),
            error("b.rule", "docs/a.md"),
            error("b.rule", "docs/z.md"),
        ];
        let p = plan_of(&findings, &[], &Filters::default());
        let got: Vec<(&str, &str, usize)> = p
            .groups
            .iter()
            .map(|g| (g.scope.as_str(), g.rule.as_str(), g.findings.len()))
            .collect();
        assert_eq!(
            got,
            [
                ("docs/a.md", "b.rule", 1),
                ("docs/z.md", "a.rule", 1),
                ("docs/z.md", "b.rule", 2)
            ]
        );
    }

    #[test]
    fn only_findings_at_or_above_fail_on_are_candidates_and_info_is_never_listed() {
        let findings = [
            finding("r", FindingSeverity::Info, "a.md", None),
            finding("r", FindingSeverity::Warning, "b.md", None),
            finding("r", FindingSeverity::Error, "c.md", None),
        ];
        let warn = plan(
            &findings,
            FailOn::Warning,
            (&[], &HashSet::new()),
            "2026-11-13",
            &Filters::default(),
        );
        assert_eq!(warn.groups.len(), 2);
        let err = plan(
            &findings,
            FailOn::Error,
            (&[], &HashSet::new()),
            "2026-11-13",
            &Filters::default(),
        );
        assert_eq!(err.groups.len(), 1);
        assert_eq!(err.groups[0].scope, "c.md");
        assert!(warn.skipped.is_empty() && err.skipped.is_empty());
    }

    #[test]
    fn a_waived_finding_is_not_a_candidate() {
        let mut f = error("r", "a.md");
        f.waived = Some("docs/waiver/WAIVER-1-x.md".into());
        assert!(plan_of(&[f], &[], &Filters::default()).groups.is_empty());
    }

    #[test]
    fn a_finding_of_waiver_matches_nothing_is_skipped_with_its_reason() {
        let f = error("waiver.matches-nothing", "docs/waiver/WAIVER-1-x.md");
        let p = plan_of(&[f], &[], &Filters::default());
        assert!(p.groups.is_empty());
        assert_eq!(p.skipped.len(), 1);
        assert_eq!(p.skipped[0].reason, SkipReason::MatchesNothing);
    }

    #[test]
    fn a_finding_with_no_file_is_skipped_with_no_file() {
        let p = plan_of(&[error("r", "")], &[], &Filters::default());
        assert!(p.groups.is_empty());
        assert_eq!(p.skipped[0].reason, SkipReason::NoFile);
        assert_eq!(p.skipped[0].file, None);
    }

    #[test]
    fn a_waiver_with_the_same_rule_and_scope_skips_the_finding_and_is_named() {
        let waivers = [suppression("r", "a.md", None)];
        let p = plan_of(&[error("r", "a.md")], &waivers, &Filters::default());
        assert_eq!(p.skipped[0].reason, SkipReason::AlreadyWaived);
        assert_eq!(
            p.skipped[0].waiver.as_deref(),
            Some("docs/waiver/WAIVER-1-x.md")
        );
        // Another scope, another rule, or a `*` scope is not the same waiver.
        for other in [
            suppression("r", "b.md", None),
            suppression("other", "a.md", None),
            suppression("r", "*", None),
        ] {
            let p = plan_of(&[error("r", "a.md")], &[other], &Filters::default());
            assert_eq!(p.groups.len(), 1);
        }
    }

    #[test]
    fn an_expired_waiver_for_the_same_rule_and_scope_skips_with_expired() {
        let waivers = [suppression("r", "a.md", Some("2026-01-01"))];
        let p = plan_of(&[error("r", "a.md")], &waivers, &Filters::default());
        assert_eq!(p.skipped[0].reason, SkipReason::Expired);
        assert!(p.groups.is_empty());
    }

    #[test]
    fn a_variant_its_rule_declares_unwaivable_is_skipped() {
        let mut f = Finding::new(
            "status.delivery-agreement",
            FindingSeverity::Error,
            "docs/m.md",
            ActionSource::new("done-with-unresolved-items"),
            "x",
        );
        f.line = None;
        let p = plan_of(&[f], &[], &Filters::default());
        assert_eq!(p.skipped[0].reason, SkipReason::Unwaivable);
        assert!(p.groups.is_empty());
    }

    #[test]
    fn rule_and_path_filters_narrow_and_unmatched_ones_are_listed() {
        let findings = [
            error("a.rule", "docs/x/one.md"),
            error("b.rule", "docs/x/one.md"),
            error("a.rule", "docs/y/two.md"),
        ];
        let filters = Filters {
            rules: vec!["a.rule".into(), "c.rule".into()],
            paths: vec![
                PathBuf::from("docs/x"),
                PathBuf::from("docs/y/two.md"),
                PathBuf::from("docs/none"),
            ],
        };
        let p = plan_of(&findings, &[], &filters);
        let got: Vec<(&str, &str)> = p
            .groups
            .iter()
            .map(|g| (g.scope.as_str(), g.rule.as_str()))
            .collect();
        assert_eq!(
            got,
            [("docs/x/one.md", "a.rule"), ("docs/y/two.md", "a.rule")]
        );
        let unmatched: Vec<(FilterKind, &str)> = p
            .unmatched
            .iter()
            .map(|u| (u.kind, u.value.as_str()))
            .collect();
        assert_eq!(
            unmatched,
            [
                (FilterKind::Rule, "c.rule"),
                (FilterKind::Path, "docs/none")
            ]
        );
    }

    #[test]
    fn a_path_filter_is_a_whole_component_prefix_not_a_text_prefix() {
        let p = plan_of(
            &[error("r", "docs/xy/one.md")],
            &[],
            &Filters {
                rules: Vec::new(),
                paths: vec![PathBuf::from("docs/x")],
            },
        );
        assert!(p.groups.is_empty());
        assert_eq!(p.unmatched.len(), 1);
    }

    #[test]
    fn findings_the_drafts_resolve_are_split_off_group_by_group() {
        let p = plan_of(
            &[
                error("gone", "a.md"),
                error("kept", "a.md"),
                error("kept", "b.md"),
            ],
            &[],
            &Filters::default(),
        );
        // After the drafts exist only `kept` in `b.md` remains of the three.
        let after = [error("kept", "b.md")];
        let d = drop_resolved(p.groups, &after);
        assert_eq!(d.kept.len(), 1);
        assert_eq!(d.kept[0].scope, "b.md");
        let gone: Vec<(&str, &str)> = d
            .resolved
            .iter()
            .map(|g| (g.rule.as_str(), g.scope.as_str()))
            .collect();
        assert_eq!(gone, [("gone", "a.md"), ("kept", "a.md")]);
        let skipped = skipped_groups(&d.resolved, SkipReason::ResolvedByDrafts);
        assert_eq!(skipped.len(), 2);
        assert_eq!(skipped[0].file.as_deref(), Some("a.md"));
    }

    #[test]
    fn a_group_that_loses_only_some_findings_is_kept_with_the_rest() {
        let mut one = error("r", "a.md");
        one.message = "first".into();
        let mut two = error("r", "a.md");
        two.message = "second".into();
        let p = plan_of(&[one, two.clone()], &[], &Filters::default());
        let d = drop_resolved(p.groups, &[two]);
        assert_eq!(d.kept[0].findings.len(), 1);
        assert_eq!(d.kept[0].findings[0].message, "second");
        assert_eq!(d.resolved[0].findings[0].message, "first");
    }

    #[test]
    fn merged_groups_join_by_rule_and_file_and_stay_in_order() {
        let g = |rule: &str, scope: &str| Group {
            rule: rule.into(),
            scope: scope.into(),
            findings: vec![error(rule, scope)],
        };
        let merged = merge_groups(vec![g("b", "z.md")], vec![g("a", "a.md"), g("b", "z.md")]);
        let got: Vec<(&str, usize)> = merged
            .iter()
            .map(|g| (g.scope.as_str(), g.findings.len()))
            .collect();
        assert_eq!(got, [("a.md", 1), ("z.md", 2)]);
    }

    #[test]
    fn a_finding_inside_a_waiver_record_is_skipped_so_drafts_are_not_drafted_for() {
        let records: HashSet<PathBuf> = [PathBuf::from("docs/waiver/WAIVER-1-x.md")].into();
        let p = plan(
            &[error("field.pending", "docs/waiver/WAIVER-1-x.md")],
            FailOn::Warning,
            (&[], &records),
            "2026-11-13",
            &Filters::default(),
        );
        assert!(p.groups.is_empty());
        assert_eq!(p.skipped[0].reason, SkipReason::WaiverRecord);
    }

    #[test]
    fn a_finding_line_names_the_file_and_the_line_when_there_is_one() {
        let with = finding("r", FindingSeverity::Error, "a.md", Some(12));
        let without = error("r", "a.md");
        assert_eq!(finding_line(&with), "a.md:12 r says so");
        assert_eq!(finding_line(&without), "a.md r says so");
    }

    #[test]
    fn the_reason_is_the_first_whole_pending_value_else_pending() {
        let declared = choose_pending(&st_with(Some(vec![
            PendingEntry::Pattern(PendingPattern {
                pattern: "^x".into(),
            }),
            PendingEntry::Value(" Needs review ".into()),
            PendingEntry::Value("Later".into()),
        ])))
        .unwrap();
        assert_eq!(declared.value, "Needs review");
        assert_eq!(declared.source, PendingSource::Declared);
        let fallback = choose_pending(&st_with(None)).unwrap();
        assert_eq!(fallback.value, "Pending");
        assert_eq!(fallback.source, PendingSource::Default);
    }

    #[test]
    fn a_type_that_does_not_require_reason_is_refused() {
        let mut st = st_with(None);
        st.effective_requires = vec!["Expires".into()];
        let err = choose_pending(&st).unwrap_err();
        assert!(err.contains("effective_requires"), "{err}");
    }

    #[test]
    fn pending_that_holds_only_patterns_is_refused() {
        let err = choose_pending(&st_with(Some(vec![PendingEntry::Pattern(
            PendingPattern {
                pattern: "^x".into(),
            },
        )])))
        .unwrap_err();
        assert!(err.contains("only patterns"), "{err}");
    }

    #[test]
    fn a_value_the_type_reads_as_present_is_refused() {
        let st = st_with(None);
        let err = ensure_not_present(&st, "Fixed upstream in 1.2").unwrap_err();
        assert!(err.contains("as a real Reason"), "{err}");
        // The values the command can choose are all read as pending.
        assert!(ensure_not_present(&st, "Pending").is_ok());
        assert!(ensure_not_present(&st, "TBD").is_ok());
        // `pending: []` declares nothing pending, and the default vocabulary
        // still reads a whole `Pending` as pending, so the fallback is safe.
        let empty = choose_pending(&st_with(Some(Vec::new()))).unwrap();
        assert_eq!(empty.source, PendingSource::Default);
    }

    #[test]
    fn a_type_with_neither_permanent_nor_expires_required_is_refused_naming_both_fixes() {
        let mut st = st_with(None);
        st.effective_requires = vec!["Reason".into()];
        let err = choose_pending(&st).unwrap_err();
        assert!(err.contains("declare `permanent`"), "{err}");
        assert!(err.contains("add Expires to `effective_requires`"), "{err}");
        // Declaring `permanent` lifts the refusal.
        st.permanent = Some(Vec::new());
        assert!(choose_pending(&st).is_ok());
    }
}
