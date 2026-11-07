//! A suppression record -- a waiver, in this repository's word -- is a
//! first-class record (RFC-0015, ADR-0011): reviewable,
//! authored, with a required reason and an optional expiry -- never a
//! config-level ignore list. A waived finding is listed, never omitted;
//! `status`/`blocking` are computed from non-waived findings only.

use crate::config::{CiteRequirement, Config, PermanentDecl};
use crate::field_state::{classify_against_template, classify_with};
use crate::record::Record;
use crate::report::Finding;
use crate::status_model::StatusDecl;
use crate::FieldState;

/// What the config declares about the suppression type: its name, the fields
/// a suppression must carry to take effect (`effective_requires`, `BUG-177`)
/// and the type's pending vocabulary, which decides what counts as a pending
/// value there.
#[derive(Debug, Clone, Default)]
pub struct SuppressionType {
    pub name: String,
    pub effective_requires: Vec<String>,
    pub declared: Option<StatusDecl>,
    /// The type's template header as `field -> value` (read by the caller from
    /// `.urzua/templates/<type>.md`): a required field still holding the
    /// template's text for it is a placeholder. Empty when there is no template.
    pub template: std::collections::HashMap<String, String>,
    /// Where a suppression without `Expires` may take effect (`ADR-86`).
    /// `None` is "not declared": permanence is allowed for any rule, as it
    /// always was. `Some` closes permanence for every rule it does not list.
    pub permanent: Option<Vec<PermanentDecl>>,
}

impl SuppressionType {
    /// A suppression type that requires nothing beyond `Rule` and `Scope`.
    pub fn named(name: &str) -> Self {
        SuppressionType {
            name: name.to_string(),
            ..Default::default()
        }
    }

    pub fn from_config(config: &Config) -> Self {
        let name = config.suppression_type();
        let declared = config.record_types.get(name);
        SuppressionType {
            name: name.to_string(),
            effective_requires: declared
                .and_then(|t| t.effective_requires.clone())
                .unwrap_or_default(),
            declared: declared.and_then(|t| t.status_decl()),
            template: Default::default(),
            permanent: declared.and_then(|t| t.permanent.clone()),
        }
    }

    /// The same type, judging a placeholder against the template's text.
    pub fn with_template(mut self, template: &str) -> Self {
        self.template = crate::new_record::template_header_values(template);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suppression {
    pub id: String,
    pub rule: String,
    /// A file path, or `"*"` for every file the rule examines.
    pub scope: String,
    /// `YYYY-MM-DD`. `None` means permanent (RFC-0015 §1: a structural
    /// exception, not merely one nobody got around to fixing).
    pub expires: Option<String>,
    /// The `effective_requires` fields this suppression lacks: absent, blank,
    /// a placeholder or pending. Empty means it takes effect (`BUG-177`).
    pub unmet: Vec<String>,
    /// Why each `unmet` field is unmet, in the same order: blank (absent or
    /// empty), a placeholder, or pending. Lets the report say which, rather
    /// than calling a field that is present "missing".
    pub unmet_states: Vec<FieldState>,
    /// Whether the type's `permanent` declaration lets this suppression stand
    /// without an `Expires` (`ADR-86`).
    pub permanence: Permanence,
}

// Implements: ADR-86 (a waiver without Expires is governed by the declared `permanent` vocabulary)
/// How a suppression stands under the type's `permanent` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Permanence {
    /// Not governed: it has an `Expires`, or the type declares no `permanent`.
    Ungoverned,
    /// Permanent, and allowed under this category.
    Allowed { category: String },
    /// Permanent, and not allowed; it takes effect for nothing.
    Refused(PermanenceRefusal),
}

/// The unmet requirement of a permanent suppression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermanenceRefusal {
    /// The type declares `permanent` and lists no entry for the rule.
    RuleNotListed,
    /// The category field is absent, blank, a placeholder or pending.
    CategoryMissing { field: String, allowed: Vec<String> },
    /// The category field holds a value outside the rule's vocabulary.
    CategoryUnlisted {
        field: String,
        value: String,
        allowed: Vec<String>,
    },
    /// The category requires a citation and the cite field holds none.
    CiteMissing { field: String, category: String },
    /// The category requires a citation and it resolves to no record.
    CiteUnresolved {
        field: String,
        value: String,
        category: String,
    },
}

impl PermanenceRefusal {
    /// The header field the requirement is about, for the finding's action.
    pub fn field(&self) -> &str {
        match self {
            PermanenceRefusal::RuleNotListed => "Expires",
            PermanenceRefusal::CategoryMissing { field, .. }
            | PermanenceRefusal::CategoryUnlisted { field, .. }
            | PermanenceRefusal::CiteMissing { field, .. }
            | PermanenceRefusal::CiteUnresolved { field, .. } => field,
        }
    }

    /// What is unmet, in the words of the finding.
    pub fn describe(&self, rule: &str) -> String {
        match self {
            PermanenceRefusal::RuleNotListed => format!(
                "it has no Expires and its type declares `permanent` without an entry for '{rule}', so permanence is not allowed for that rule"
            ),
            PermanenceRefusal::CategoryMissing { field, allowed } => format!(
                "it has no Expires and no {field}; `permanent` allows it for '{rule}' only under one of: {}",
                allowed.join(", ")
            ),
            PermanenceRefusal::CategoryUnlisted {
                field,
                value,
                allowed,
            } => format!(
                "it has no Expires and its {field} '{value}' is not one `permanent` allows for '{rule}': {}",
                allowed.join(", ")
            ),
            PermanenceRefusal::CiteMissing { field, category } => format!(
                "its category '{category}' requires a citation and {field} holds none"
            ),
            PermanenceRefusal::CiteUnresolved {
                field,
                value,
                category,
            } => format!(
                "its category '{category}' requires a citation and {field} '{value}' resolves to no record"
            ),
        }
    }
}

impl Suppression {
    /// A waiver with a passed expiry is not a waiver (RFC-0015 §3): treated
    /// as though it doesn't exist, with no separate "expired" state to
    /// configure.
    ///
    /// The `<=` is lexical, which only holds once both sides are known
    /// `YYYY-MM-DD`: any other value sorts above a real date
    /// (`"2026-09-16" <= "soon"`) and would suppress findings forever. A
    /// non-date expiry expires immediately instead -- this mechanism may
    /// fail toward more findings, never fewer.
    pub fn is_active(&self, today: &str) -> bool {
        match &self.expires {
            None => true,
            Some(expires) => is_iso_date(expires) && today <= expires.as_str(),
        }
    }

    /// Whether it carries every field the type requires for it to take effect.
    /// Separate from `is_active`: an ineffective suppression is reported as
    /// such, where an expired one is reported as expired.
    pub fn is_effective(&self) -> bool {
        self.unmet.is_empty() && !matches!(self.permanence, Permanence::Refused(_))
    }

    /// The category it stands permanent under, when it does.
    pub fn permanent_category(&self) -> Option<&str> {
        match &self.permanence {
            Permanence::Allowed { category } => Some(category),
            _ => None,
        }
    }

    pub fn covers(&self, finding: &Finding) -> bool {
        if self.rule != finding.rule {
            return false;
        }
        self.scope == "*" || finding.file.to_string_lossy() == self.scope
    }
}

/// Parse suppression records out of the discovered corpus: the records of the
/// type named `suppression_type`, which the config declares with
/// `role: suppression` (`Config::suppression_type`) -- the engine knows no
/// type name. A suppression missing
/// `Rule` or `Scope` is skipped rather than crashing the run -- a malformed
/// waiver should never take down `check` itself, and an un-parseable waiver
/// simply waives nothing, which fails toward more findings, not fewer.
pub fn load_suppressions(
    records: &[Record],
    suppression_type: &SuppressionType,
) -> Vec<Suppression> {
    let known: std::collections::HashSet<crate::values::RecordId> = records
        .iter()
        .filter_map(crate::rules::record_id)
        .map(|id| crate::values::RecordId::new(&id))
        .collect();
    records
        .iter()
        .filter(|r| r.record_type == suppression_type.name)
        .filter_map(|r| {
            // Reserved keys (Header::get_reserved), not adopter vocabulary --
            // a waiver record spelling these in another case has made a typo,
            // not declared a different field, and reading them exactly would
            // drop a waiver that should apply.
            let rule = r.header.get_reserved("Rule")?.to_string();
            let scope = r.header.get_reserved("Scope")?.to_string();
            let expires = r.header.get_reserved("Expires").map(|s| s.to_string());
            // A prose field is a placeholder only when its whole trimmed value
            // is the template's text for it or a declared pending value; the
            // word "pending" inside a real sentence is a real reason.
            let (unmet, unmet_states): (Vec<String>, Vec<FieldState>) = suppression_type
                .effective_requires
                .iter()
                .filter_map(|field| {
                    let mut state = classify_against_template(
                        r.header.get(field),
                        suppression_type.template.get(field).map(String::as_str),
                        suppression_type.declared.as_ref(),
                    );
                    // The default vocabulary still applies to the whole value:
                    // a Reason that is just `TBD` or `TODO` is a placeholder.
                    if state == FieldState::Present {
                        if let Some(default) = r
                            .header
                            .get(field)
                            .and_then(crate::field_state::whole_value_default_state)
                        {
                            state = default;
                        }
                    }
                    (state != FieldState::Present).then(|| (field.clone(), state))
                })
                .unzip();
            let permanence =
                judge_permanence(r, &rule, expires.is_some(), suppression_type, &known);
            Some(Suppression {
                id: r.path.to_string_lossy().to_string(),
                rule,
                scope,
                expires,
                unmet,
                unmet_states,
                permanence,
            })
        })
        .collect()
}

/// Whether the type's `permanent` declaration lets a suppression without
/// `Expires` take effect (`ADR-86`): its rule has an entry, its reason field
/// holds one of the entry's categories, and its cite field resolves to a record
/// when the category requires one. A suppression with an `Expires`, or a type
/// that declares nothing, is not governed here.
fn judge_permanence(
    record: &Record,
    rule: &str,
    has_expiry: bool,
    suppression_type: &SuppressionType,
    known: &std::collections::HashSet<crate::values::RecordId>,
) -> Permanence {
    use PermanenceRefusal::*;
    let Some(entries) = &suppression_type.permanent else {
        return Permanence::Ungoverned;
    };
    if has_expiry {
        return Permanence::Ungoverned;
    }
    let Some(entry) = entries.iter().find(|e| e.rule == rule) else {
        return Permanence::Refused(RuleNotListed);
    };
    let field = entry.reason_field.clone();
    let allowed: Vec<String> = entry.categories.iter().map(|c| c.name.clone()).collect();
    let value = record.header.get(&field);
    if classify_with(value, suppression_type.declared.as_ref()) != FieldState::Present {
        return Permanence::Refused(CategoryMissing { field, allowed });
    }
    let value = value.unwrap_or_default().trim();
    let Some(category) = entry.categories.iter().find(|c| c.name == value) else {
        return Permanence::Refused(CategoryUnlisted {
            field,
            value: value.to_string(),
            allowed,
        });
    };
    if category.cite == Some(CiteRequirement::Required) {
        // Validated at load: a category requiring a citation has a cite field.
        let cite_field = entry.cite_field.clone().unwrap_or_default();
        let cited = record.header.get(&cite_field).unwrap_or_default();
        let references = crate::rules::extract_references(cited);
        if references.is_empty() {
            return Permanence::Refused(CiteMissing {
                field: cite_field,
                category: category.name.clone(),
            });
        }
        if let Some(missing) = references
            .iter()
            .find(|r| !known.contains(&crate::values::RecordId::new(r)))
        {
            return Permanence::Refused(CiteUnresolved {
                field: cite_field,
                value: missing.clone(),
                category: category.name.clone(),
            });
        }
    }
    Permanence::Allowed {
        category: category.name.clone(),
    }
}

/// Calendar validity (a 31st of February) is deliberately unchecked: it still
/// orders correctly against a real date, which is all `is_active` asks.
fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && [0, 1, 2, 3, 5, 6, 8, 9]
            .iter()
            .all(|&i| bytes[i].is_ascii_digit())
}

/// Mark each finding covered by an active waiver. Findings are never
/// removed -- only annotated -- so the caller's `blocking` computation must
/// itself skip waived findings; this function does not touch severity.
pub fn apply_suppressions(findings: &mut [Finding], waivers: &[Suppression], today: &str) {
    for finding in findings.iter_mut() {
        for waiver in waivers
            .iter()
            .filter(|w| w.is_active(today) && w.is_effective())
        {
            if waiver.covers(finding) {
                finding.waived = Some(waiver.id.clone());
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{ActionSource, FindingSeverity};
    use std::path::PathBuf;

    fn finding(rule: &str, file: &str) -> Finding {
        Finding {
            rule: rule.to_string(),
            severity: FindingSeverity::Error,
            file: PathBuf::from(file),
            line: None,
            message: "x".to_string(),
            waived: None,
            declared_severity: None,
            suggested_action: None,
            action_source: ActionSource::default(),
        }
    }

    #[test]
    fn a_waiver_with_lowercase_keys_still_loads_observed_failing() {
        // Header::get went exact under ADR-57, and Rule/Scope/Expires are not
        // adopter vocabulary -- there is exactly one field the engine calls
        // Rule, so a hand-typed `rule:` is a typo, not a second field, and
        // reading it exactly silently dropped an active waiver.
        let r = Record::parse(
            std::path::PathBuf::from("docs/waivers/WAIVER-1-x.md"),
            "waiver".to_string(),
            "---
rule: header.required-fields
scope: docs/adr/ADR-1-x.md
---
",
        );
        let waivers = load_suppressions(&[r], &SuppressionType::named("waiver"));
        assert_eq!(
            waivers.len(),
            1,
            "a case-differing key must not drop the waiver"
        );
        assert_eq!(waivers[0].rule, "header.required-fields");
        assert_eq!(waivers[0].scope, "docs/adr/ADR-1-x.md");
    }

    #[test]
    fn a_permanent_waiver_is_always_active() {
        let w = Suppression {
            id: "w1".into(),
            rule: "r".into(),
            scope: "*".into(),
            expires: None,
            unmet: Vec::new(),
            unmet_states: Vec::new(),
            permanence: Permanence::Ungoverned,
        };
        assert!(w.is_active("2020-01-01"));
        assert!(w.is_active("2099-01-01"));
    }

    #[test]
    fn an_expired_waiver_is_not_active() {
        let w = Suppression {
            id: "w1".into(),
            rule: "r".into(),
            scope: "*".into(),
            expires: Some("2026-01-01".into()),
            unmet: Vec::new(),
            unmet_states: Vec::new(),
            permanence: Permanence::Ungoverned,
        };
        assert!(w.is_active("2025-12-31"));
        assert!(!w.is_active("2026-01-02"));
    }

    #[test]
    fn an_unparseable_expiry_does_not_suppress_observed_failing() {
        // `today <= expires` is a string compare, so any value sorting above
        // a date -- "soon", "TBD", or a single-digit month -- suppresses
        // forever. A waiver nobody can expire is the one failure this
        // mechanism must not have.
        for bad in ["soon", "TBD", "Q4", "2026-9-16"] {
            let w = Suppression {
                id: "w1".into(),
                rule: "r".into(),
                scope: "*".into(),
                expires: Some(bad.into()),
                unmet: Vec::new(),
                unmet_states: Vec::new(),
                permanence: Permanence::Ungoverned,
            };
            assert!(
                !w.is_active("2026-09-16"),
                "{bad:?} must not act as a permanent waiver"
            );
        }
    }

    #[test]
    fn a_matching_active_waiver_marks_the_finding_waived_not_removed() {
        let mut findings = vec![finding("field.quality", "docs/adr/0001-x.md")];
        let waivers = vec![Suppression {
            id: "docs/waiver/0001-x.md".into(),
            rule: "field.quality".into(),
            scope: "docs/adr/0001-x.md".into(),
            expires: None,
            unmet: Vec::new(),
            unmet_states: Vec::new(),
            permanence: Permanence::Ungoverned,
        }];
        apply_suppressions(&mut findings, &waivers, "2026-09-04");

        assert_eq!(findings.len(), 1, "waiving must never remove a finding");
        assert_eq!(
            findings[0].waived,
            Some("docs/waiver/0001-x.md".to_string())
        );
    }

    #[test]
    fn a_wildcard_scope_covers_every_file_for_that_rule() {
        let mut findings = vec![
            finding("field.quality", "docs/adr/0001-x.md"),
            finding("field.quality", "docs/adr/0002-y.md"),
            finding("pointer.resolution", "docs/adr/0002-y.md"),
        ];
        let waivers = vec![Suppression {
            id: "w1".into(),
            rule: "field.quality".into(),
            scope: "*".into(),
            expires: None,
            unmet: Vec::new(),
            unmet_states: Vec::new(),
            permanence: Permanence::Ungoverned,
        }];
        apply_suppressions(&mut findings, &waivers, "2026-09-04");

        assert!(findings[0].waived.is_some());
        assert!(findings[1].waived.is_some());
        assert!(
            findings[2].waived.is_none(),
            "a different rule must not be covered"
        );
    }

    #[test]
    fn an_expired_waiver_does_not_cover_anything() {
        let mut findings = vec![finding("field.quality", "docs/adr/0001-x.md")];
        let waivers = vec![Suppression {
            id: "w1".into(),
            rule: "field.quality".into(),
            scope: "*".into(),
            expires: Some("2020-01-01".into()),
            unmet: Vec::new(),
            unmet_states: Vec::new(),
            permanence: Permanence::Ungoverned,
        }];
        apply_suppressions(&mut findings, &waivers, "2026-09-04");

        assert!(findings[0].waived.is_none());
    }

    fn reason_record(reason: Option<&str>) -> Record {
        let reason = reason.map(|r| format!("Reason: {r}\n")).unwrap_or_default();
        Record::parse(
            std::path::PathBuf::from("docs/waiver/WAIVER-1-x.md"),
            "waiver".to_string(),
            &format!("---\nRule: field.quality\nScope: '*'\n{reason}---\n"),
        )
    }

    fn requiring_reason() -> SuppressionType {
        SuppressionType {
            name: "waiver".into(),
            effective_requires: vec!["Reason".into()],
            ..Default::default()
        }
    }

    /// `BUG-177`: a suppression lacking a declared field, or holding a blank,
    /// the template's placeholder text or a declared pending value, is loaded
    /// (so it can be reported) but does not suppress.
    #[test]
    fn a_suppression_lacking_a_required_field_does_not_suppress_observed_failing() {
        let mut kind = requiring_reason()
            .with_template("---\nRule: x\nReason: why this exception is accepted\n---\n");
        kind.declared = Some(pending_decl());
        for (case, reason, state) in [
            ("absent", None, FieldState::Blank),
            ("blank", Some("''"), FieldState::Blank),
            (
                "template text",
                Some("why this exception is accepted"),
                FieldState::Placeholder,
            ),
            ("pending", Some("Pending"), FieldState::Pending),
            (
                "pending, padded",
                Some("'  Pending  '"),
                FieldState::Pending,
            ),
        ] {
            let loaded = load_suppressions(&[reason_record(reason)], &kind);
            assert_eq!(loaded.len(), 1, "{case}");
            assert_eq!(loaded[0].unmet, vec!["Reason".to_string()], "{case}");
            assert_eq!(loaded[0].unmet_states, vec![state], "{case}");
            let mut findings = vec![finding("field.quality", "docs/adr/0001-x.md")];
            apply_suppressions(&mut findings, &loaded, "2026-11-05");
            assert!(findings[0].waived.is_none(), "{case}");
        }
    }

    /// A Reason that is entirely a default placeholder word is not a real
    /// reason, whatever the case or padding; a sentence holding the word is.
    #[test]
    fn a_reason_that_is_only_a_default_placeholder_word_does_not_suppress_observed_failing() {
        for kind in [
            requiring_reason(),
            requiring_reason().with_template("---\nRule: x\nReason: why\n---\n"),
        ] {
            for reason in ["TBD", "tbd", "TODO", "'  Todo  '"] {
                let loaded = load_suppressions(&[reason_record(Some(reason))], &kind);
                assert_eq!(loaded[0].unmet, vec!["Reason".to_string()], "{reason}");
                assert_eq!(loaded[0].unmet_states, vec![FieldState::Placeholder], "{reason}");
                assert!(!loaded[0].is_effective(), "{reason}");
            }
            let loaded = load_suppressions(&[reason_record(Some("TBD until the upstream fix ships"))], &kind);
            assert!(loaded[0].is_effective());
        }
    }

    fn pending_decl() -> StatusDecl {
        yaml_serde::from_str("values: [Open]\npending: [pending]\n").unwrap()
    }

    /// The word "pending" inside real prose is a real reason; only a whole value
    /// that is the declared pending value or the template's text is a placeholder.
    #[test]
    fn a_reason_containing_the_word_pending_in_prose_suppresses_observed_failing() {
        for kind in [
            requiring_reason(),
            SuppressionType {
                declared: Some(pending_decl()),
                ..requiring_reason()
            },
        ] {
            for reason in [
                "pending the upstream fix",
                "Deferred until the pending refactor lands",
                "TBD until the upstream fix ships",
            ] {
                let loaded = load_suppressions(&[reason_record(Some(reason))], &kind);
                assert!(loaded[0].is_effective(), "{reason}: {:?}", loaded[0]);
                assert!(loaded[0].unmet_states.is_empty(), "{reason}");
            }
        }
    }

    #[test]
    fn a_suppression_carrying_the_required_field_suppresses() {
        let loaded = load_suppressions(
            &[reason_record(Some("a structural exception"))],
            &requiring_reason(),
        );
        assert!(loaded[0].is_effective());
        let mut findings = vec![finding("field.quality", "docs/adr/0001-x.md")];
        apply_suppressions(&mut findings, &loaded, "2026-11-05");
        assert!(findings[0].waived.is_some());
    }

    #[test]
    fn without_effective_requires_a_reasonless_suppression_suppresses() {
        let loaded = load_suppressions(&[reason_record(None)], &SuppressionType::named("waiver"));
        assert!(loaded[0].is_effective());
    }

    fn permanent_type() -> SuppressionType {
        let entry = |rule: &str| PermanentDecl {
            rule: rule.into(),
            reason_field: "Category".into(),
            categories: vec![
                crate::config::PermanentCategory {
                    name: "perf".into(),
                    cite: None,
                },
                crate::config::PermanentCategory {
                    name: "decided".into(),
                    cite: Some(CiteRequirement::Required),
                },
            ],
            cite_field: Some("Justified-by".into()),
        };
        SuppressionType {
            name: "waiver".into(),
            permanent: Some(vec![entry("field.quality")]),
            ..Default::default()
        }
    }

    fn waiver_with(rule: &str, extra: &str) -> Record {
        Record::parse(
            std::path::PathBuf::from("docs/waiver/WAIVER-1-x.md"),
            "waiver".to_string(),
            &format!("---\nRule: {rule}\nScope: '*'\n{extra}---\n"),
        )
    }

    fn rfc(n: u32) -> Record {
        Record::parse(
            std::path::PathBuf::from(format!("docs/rfc/RFC-{n}-x.md")),
            "rfc".to_string(),
            "---\nStatus: Accepted\n---\n",
        )
    }

    fn permanence_of(rule: &str, extra: &str) -> Permanence {
        let corpus = [waiver_with(rule, extra), rfc(35)];
        load_suppressions(&corpus, &permanent_type())
            .remove(0)
            .permanence
    }

    #[test]
    fn a_permanent_waiver_stands_only_under_a_listed_category_observed_failing() {
        assert_eq!(
            permanence_of("field.quality", "Category: perf\n"),
            Permanence::Allowed {
                category: "perf".into()
            }
        );
        assert!(matches!(
            permanence_of("field.quality", "Category: whim\n"),
            Permanence::Refused(PermanenceRefusal::CategoryUnlisted { .. })
        ));
        for blank in ["", "Category: ''\n", "Category: TBD\n"] {
            assert!(
                matches!(
                    permanence_of("field.quality", blank),
                    Permanence::Refused(PermanenceRefusal::CategoryMissing { .. })
                ),
                "{blank:?}"
            );
        }
        assert_eq!(
            permanence_of("other.rule", "Category: perf\n"),
            Permanence::Refused(PermanenceRefusal::RuleNotListed)
        );
    }

    #[test]
    fn a_category_requiring_a_citation_needs_a_resolving_one() {
        assert!(matches!(
            permanence_of("field.quality", "Category: decided\n"),
            Permanence::Refused(PermanenceRefusal::CiteMissing { .. })
        ));
        assert!(matches!(
            permanence_of("field.quality", "Category: decided\nJustified-by: RFC-9\n"),
            Permanence::Refused(PermanenceRefusal::CiteUnresolved { .. })
        ));
        // A padded spelling resolves to the same record.
        for cite in ["RFC-35", "RFC-0035"] {
            assert_eq!(
                permanence_of(
                    "field.quality",
                    &format!("Category: decided\nJustified-by: {cite}\n")
                ),
                Permanence::Allowed {
                    category: "decided".into()
                },
                "{cite}"
            );
        }
    }

    #[test]
    fn an_expiry_or_an_undeclared_type_leaves_permanence_ungoverned() {
        assert_eq!(
            permanence_of("other.rule", "Expires: 2099-01-01\n"),
            Permanence::Ungoverned
        );
        let corpus = [waiver_with("other.rule", "")];
        let loaded = load_suppressions(&corpus, &SuppressionType::named("waiver"));
        assert_eq!(loaded[0].permanence, Permanence::Ungoverned);
        assert!(loaded[0].is_effective());
    }

    #[test]
    fn a_refused_permanent_waiver_suppresses_nothing() {
        let corpus = [waiver_with("field.quality", "Category: whim\n")];
        let loaded = load_suppressions(&corpus, &permanent_type());
        let mut findings = vec![finding("field.quality", "docs/adr/0001-x.md")];
        apply_suppressions(&mut findings, &loaded, "2026-11-05");
        assert!(findings[0].waived.is_none());
    }
}
