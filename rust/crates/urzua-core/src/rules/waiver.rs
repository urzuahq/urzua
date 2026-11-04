use super::*;
use crate::waiver::Waiver;

/// `waiver.matches-nothing` (`MILE-168`): a waiver that no longer does
/// anything is reported, so a stale exception does not stay on file hiding that
/// it is unneeded (and free to cover a future finding by accident).
///
/// Three states, each a finding on the waiver record:
///
/// - **Expired**: its `Expires` has passed, or is not a real `YYYY-MM-DD`
///   date. `apply_waivers` already ignores it, so its finding resurfaces; this
///   names the record that is left behind. Reported once as expired, never
///   also as unmatched.
/// - **Unknown rule**: it names a rule this build does not ship, so it can
///   never match anything.
/// - **Unmatched**: it is active, its rule ran in this invocation over the
///   whole corpus, and no finding of that rule falls in its scope.
///
/// A waiver is judged only on what the run could see. `reached` says whether a
/// path was examined (the whole corpus, or the changed set of a `--base` run);
/// an expiry or unknown rule is about the record itself, so it needs only that.
/// "Matches no finding" needs the whole corpus *and* the target rule to have
/// run: under a path scope, a diff, or with the rule off, an absent finding
/// establishes nothing. Those waivers are eligible and unexamined, never
/// out of scope (a partial run is not a misconfiguration).
///
/// `findings` is every finding before waivers are applied.
pub fn waiver_matches_nothing(
    waivers: &[Waiver],
    findings: &[Finding],
    executed: &[RuleExecution],
    whole_corpus: bool,
    reached: impl Fn(&std::path::Path) -> bool,
    today: &str,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_WAIVER_MATCHES_NOTHING;
    let mut out = Vec::new();

    let (population, examined_records) = census_records(
        PopulationUnit::Record,
        waivers.iter().collect::<Vec<_>>(),
        |w| PathBuf::from(&w.id),
        |w| {
            let path = PathBuf::from(&w.id);
            let mut report = |message: String| {
                out.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Warning,
                    file: path.clone(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    message,
                });
                Outcome::Examined
            };
            let claimed = format!("rule '{}', scope '{}'", w.rule, w.scope);

            if !w.is_active(today) {
                if !reached(&path) {
                    return Outcome::Absent;
                }
                let expires = w.expires.as_deref().unwrap_or_default();
                return report(format!(
                    "waiver ({claimed}) is expired: Expires '{expires}' has passed or is not a \
                     YYYY-MM-DD date, so it covers nothing; remove it or set a new date"
                ));
            }
            if !ALL_RULES.contains(&w.rule.as_str()) {
                if !reached(&path) {
                    return Outcome::Absent;
                }
                return report(format!(
                    "waiver ({claimed}) names a rule this build does not have, so it can \
                     never match a finding"
                ));
            }
            let target_ran = executed
                .iter()
                .any(|e| e.rule == w.rule && e.status == RuleStatus::Ran);
            if !whole_corpus || !target_ran {
                return Outcome::Absent;
            }
            if findings.iter().any(|f| w.covers(f)) {
                return Outcome::Examined;
            }
            report(format!(
                "waiver ({claimed}) matches no finding: '{}' ran over the whole corpus and \
                 reported nothing in that scope; remove the waiver",
                w.rule
            ))
        },
    );

    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        out,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::{examined, exec_with};

    fn waiver(id: &str, rule: &str, scope: &str, expires: Option<&str>) -> Waiver {
        Waiver {
            id: id.into(),
            rule: rule.into(),
            scope: scope.into(),
            expires: expires.map(str::to_string),
        }
    }

    fn finding(rule: &str, file: &str) -> Finding {
        Finding {
            rule: rule.into(),
            severity: FindingSeverity::Error,
            file: file.into(),
            line: None,
            message: "x".into(),
            waived: None,
            declared_severity: None,
        }
    }

    fn run(
        waivers: &[Waiver],
        findings: &[Finding],
        executed: &[RuleExecution],
        whole: bool,
    ) -> (RuleExecution, Vec<Finding>) {
        waiver_matches_nothing(waivers, findings, executed, whole, |_| whole, "2026-11-04")
    }

    #[test]
    fn a_waiver_matching_no_finding_is_reported_observed_failing() {
        let w = waiver("docs/waiver/W-1.md", "field.quality", "docs/a.md", None);
        let (_, f) = run(&[w], &[], &[exec_with("field.quality", 1, 1, 0)], true);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].file, PathBuf::from("docs/waiver/W-1.md"));
        assert!(f[0].message.contains("field.quality") && f[0].message.contains("docs/a.md"));
    }

    #[test]
    fn a_waiver_matching_a_finding_is_not_reported() {
        let w = waiver("W", "field.quality", "docs/a.md", None);
        let fs = [finding("field.quality", "docs/a.md")];
        let (exec, f) = run(&[w], &fs, &[exec_with("field.quality", 1, 1, 0)], true);
        assert!(f.is_empty());
        assert_eq!(examined(&exec), 1);
    }

    #[test]
    fn a_wildcard_waiver_matches_any_finding_of_its_rule() {
        let w = waiver("W", "field.quality", "*", None);
        let fs = [finding("field.quality", "docs/z.md")];
        let ran = [exec_with("field.quality", 1, 1, 0)];
        assert!(run(std::slice::from_ref(&w), &fs, &ran, true).1.is_empty());
        assert_eq!(run(&[w], &[], &ran, true).1.len(), 1);
    }

    #[test]
    fn a_finding_of_another_rule_or_file_does_not_keep_it_alive() {
        let w = waiver("W", "field.quality", "docs/a.md", None);
        let fs = [
            finding("field.pending", "docs/a.md"),
            finding("field.quality", "docs/b.md"),
        ];
        let ran = [exec_with("field.quality", 1, 1, 0)];
        assert_eq!(run(&[w], &fs, &ran, true).1.len(), 1);
    }

    #[test]
    fn a_waiver_whose_rule_did_not_run_is_not_judged() {
        let w = waiver("W", "field.quality", "docs/a.md", None);
        let off = RuleExecution {
            rule: "field.quality".into(),
            population: None,
            status: RuleStatus::NotEnabled,
            examined_records: Vec::new(),
        };
        let (exec, f) = run(&[w.clone()], &[], &[off], true);
        assert!(f.is_empty());
        assert_eq!(examined(&exec), 0);
        assert_eq!(exec.population.unwrap().out_of_scope(), 0);
        // And absent from the executions altogether.
        assert!(run(&[w], &[], &[], true).1.is_empty());
    }

    #[test]
    fn a_partial_run_does_not_judge_a_live_waiver() {
        let w = waiver("W", "field.quality", "docs/a.md", None);
        let ran = [exec_with("field.quality", 1, 1, 0)];
        let (exec, f) = run(&[w], &[], &ran, false);
        assert!(f.is_empty(), "no finding in a partial run proves nothing");
        let p = exec.population.unwrap();
        assert_eq!((p.eligible(), p.examined(), p.out_of_scope()), (1, 0, 0));
    }

    #[test]
    fn an_expired_waiver_is_reported_once_as_expired() {
        let w = waiver("W", "field.quality", "docs/a.md", Some("2026-01-01"));
        let (_, f) = run(&[w], &[], &[exec_with("field.quality", 1, 1, 0)], true);
        assert_eq!(f.len(), 1);
        assert!(f[0].message.contains("expired"));
    }

    #[test]
    fn a_non_date_expiry_is_reported_as_expired() {
        let w = waiver("W", "field.quality", "*", Some("soon"));
        let (_, f) = run(&[w], &[], &[], true);
        assert_eq!(f.len(), 1);
        assert!(f[0].message.contains("soon"));
    }

    #[test]
    fn a_waiver_expiring_today_is_still_active() {
        let w = waiver("W", "field.quality", "*", Some("2026-11-04"));
        let fs = [finding("field.quality", "a.md")];
        let ran = [exec_with("field.quality", 1, 1, 0)];
        assert!(run(&[w], &fs, &ran, true).1.is_empty());
    }

    #[test]
    fn an_expired_waiver_is_judged_only_when_its_record_was_reached() {
        let w = waiver(
            "docs/waiver/W-1.md",
            "field.quality",
            "*",
            Some("2020-01-01"),
        );
        let (_, f) = waiver_matches_nothing(&[w], &[], &[], false, |_| false, "2026-11-04");
        assert!(f.is_empty());
    }

    #[test]
    fn a_waiver_for_an_unknown_rule_is_reported() {
        let w = waiver("W", "no.such-rule", "*", None);
        let (_, f) = run(&[w], &[], &[], true);
        assert_eq!(f.len(), 1);
        assert!(f[0].message.contains("no.such-rule"));
    }
}
