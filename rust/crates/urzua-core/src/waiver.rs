//! A waiver is a first-class record (RFC-0015, ADR-0011): reviewable,
//! authored, with a required reason and an optional expiry -- never a
//! config-level ignore list. A waived finding is listed, never omitted;
//! `status`/`blocking` are computed from non-waived findings only.

use crate::record::Record;
use crate::report::Finding;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waiver {
    pub id: String,
    pub rule: String,
    /// A file path, or `"*"` for every file the rule examines.
    pub scope: String,
    /// `YYYY-MM-DD`. `None` means permanent (RFC-0015 §1: a structural
    /// exception, not merely one nobody got around to fixing).
    pub expires: Option<String>,
}

impl Waiver {
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

    fn covers(&self, finding: &Finding) -> bool {
        if self.rule != finding.rule {
            return false;
        }
        self.scope == "*" || finding.file.to_string_lossy() == self.scope
    }
}

/// Parse waiver records out of the discovered corpus. A waiver missing
/// `Rule` or `Scope` is skipped rather than crashing the run -- a malformed
/// waiver should never take down `check` itself, and an un-parseable waiver
/// simply waives nothing, which fails toward more findings, not fewer.
pub fn load_waivers(records: &[Record]) -> Vec<Waiver> {
    records
        .iter()
        .filter(|r| r.record_type == "waiver")
        .filter_map(|r| {
            // Reserved keys (Header::get_reserved), not adopter vocabulary --
            // a waiver record spelling these in another case has made a typo,
            // not declared a different field, and reading them exactly would
            // drop a waiver that should apply.
            let rule = r.header.get_reserved("Rule")?.to_string();
            let scope = r.header.get_reserved("Scope")?.to_string();
            let expires = r.header.get_reserved("Expires").map(|s| s.to_string());
            Some(Waiver {
                id: r.path.to_string_lossy().to_string(),
                rule,
                scope,
                expires,
            })
        })
        .collect()
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
pub fn apply_waivers(findings: &mut [Finding], waivers: &[Waiver], today: &str) {
    for finding in findings.iter_mut() {
        for waiver in waivers.iter().filter(|w| w.is_active(today)) {
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
    use crate::report::FindingSeverity;
    use std::path::PathBuf;

    fn finding(rule: &str, file: &str) -> Finding {
        Finding {
            rule: rule.to_string(),
            severity: FindingSeverity::Error,
            file: PathBuf::from(file),
            line: None,
            message: "x".to_string(),
            waived: None,
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
            "> rule: header.required-fields
> scope: docs/adr/ADR-1-x.md
",
        );
        let waivers = load_waivers(&[r]);
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
        let w = Waiver {
            id: "w1".into(),
            rule: "r".into(),
            scope: "*".into(),
            expires: None,
        };
        assert!(w.is_active("2020-01-01"));
        assert!(w.is_active("2099-01-01"));
    }

    #[test]
    fn an_expired_waiver_is_not_active() {
        let w = Waiver {
            id: "w1".into(),
            rule: "r".into(),
            scope: "*".into(),
            expires: Some("2026-01-01".into()),
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
            let w = Waiver {
                id: "w1".into(),
                rule: "r".into(),
                scope: "*".into(),
                expires: Some(bad.into()),
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
        let waivers = vec![Waiver {
            id: "docs/waiver/0001-x.md".into(),
            rule: "field.quality".into(),
            scope: "docs/adr/0001-x.md".into(),
            expires: None,
        }];
        apply_waivers(&mut findings, &waivers, "2026-09-04");

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
        let waivers = vec![Waiver {
            id: "w1".into(),
            rule: "field.quality".into(),
            scope: "*".into(),
            expires: None,
        }];
        apply_waivers(&mut findings, &waivers, "2026-09-04");

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
        let waivers = vec![Waiver {
            id: "w1".into(),
            rule: "field.quality".into(),
            scope: "*".into(),
            expires: Some("2020-01-01".into()),
        }];
        apply_waivers(&mut findings, &waivers, "2026-09-04");

        assert!(findings[0].waived.is_none());
    }
}
