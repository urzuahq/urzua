/// `Ok` means "checked and clean", so it requires that some rule actually ran.
/// A run with no rule at all has established nothing, and reporting it clean is
/// indistinguishable from a corpus that passed (ADR-55). Reachable since rules
/// became opt-in: an empty or absent `rules` table.
///
/// This is the *whole* test, deliberately. Five earlier guards each asked a
/// second question here -- `records_examined > 0`, then "a record-scoped rule
/// ran", then a population-unit test -- and each was wrong in the opposite
/// direction from the last (BUG-77, BUG-81, BUG-83, BUG-84, BUG-88). The
/// fourth review established with fixtures that **no predicate over rule
/// populations satisfies BUG-84 and BUG-88 at once**: `init` writes
/// `required_fields: []` and no `pointer_fields`, so a legitimate fresh
/// adoption has nothing eligible, while a corpus no declared rule can read
/// must not read as clean.
///
/// That is two different failures forced through one mechanism. ADR-53 governs
/// *policy* -- what gets checked is the adopter's choice, so a thin config
/// earning `ok` is correct. ADR-55 governs *disclosure* -- whether the engine
/// reports honestly what it did, which is not the adopter's call. The verdict
/// keeps the first; `records_read_by_any_rule` carries the second, so a run
/// whose declared rules read no record exits 0 and says `0` where a reader
/// sees it. MILE-106's opt-in rule is what turns that disclosure into a
/// verdict for adopters who want one.
pub(crate) fn any_rule_looked(executed: &[urzua_core::report::RuleExecution]) -> bool {
    use urzua_core::report::RuleStatus;
    executed.iter().any(|e| e.status == RuleStatus::Ran)
}

/// The verdict of a `check` run, from what it established. Pure, so each
/// branch is testable without a repository.
///
/// - `blocking` settles the question first: a blocking finding is something
///   established, so it must never read `not-run` beside `blocking: true`.
/// - `empty_diff`: the run was asked about nothing (no governed record changed
///   and nothing reaches one), or the corpus holds no record yet under
///   directories that are all there. If a rule looked it is `ok`, never
///   `not-run`; the caller says which in a notice.
/// - otherwise a run that examined no record, or in which no rule looked,
///   established nothing: `not-run` (ADR-55).
/// - otherwise the verdict is the unwaived findings.
///
/// `active_findings` counts findings that are not waived.
pub(crate) fn derive_status(
    blocking: bool,
    empty_diff: bool,
    examined: usize,
    executed: &[urzua_core::report::RuleExecution],
    active_findings: usize,
) -> urzua_core::report::ReportStatus {
    use urzua_core::report::ReportStatus;
    let from_findings = if active_findings == 0 {
        ReportStatus::Ok
    } else {
        ReportStatus::FindingsPresent
    };
    if blocking {
        ReportStatus::FindingsPresent
    } else if empty_diff && any_rule_looked(executed) {
        from_findings
    } else if examined == 0 || !any_rule_looked(executed) {
        ReportStatus::NotRun
    } else {
        from_findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use urzua_core::report::{ReportStatus, RuleExecution, RuleStatus};

    fn exec(status: RuleStatus) -> RuleExecution {
        RuleExecution {
            rule: "t".to_string(),
            population: None,
            status,
            examined_records: Vec::new(),
        }
    }

    #[test]
    fn derives_each_branch() {
        let looked = [exec(RuleStatus::Ran)];
        let none_looked = [exec(RuleStatus::NotEnabled)];
        use ReportStatus::*;
        // (blocking, empty_diff, examined, executed, active, expected)
        let table = [
            (true, false, 0, &none_looked[..], 1, FindingsPresent),
            (true, true, 0, &looked, 1, FindingsPresent),
            (false, true, 0, &looked, 0, Ok),
            (false, true, 0, &looked, 2, FindingsPresent),
            (false, true, 0, &none_looked, 0, NotRun),
            (false, false, 0, &looked, 0, NotRun),
            (false, false, 3, &none_looked, 0, NotRun),
            (false, false, 3, &[], 0, NotRun),
            (false, false, 3, &looked, 0, Ok),
            (false, false, 3, &looked, 2, FindingsPresent),
            (false, false, 0, &none_looked, 2, NotRun),
        ];
        for (blocking, empty, examined, executed, active, expected) in table {
            assert_eq!(
                derive_status(blocking, empty, examined, executed, active),
                expected,
                "blocking={blocking} empty_diff={empty} examined={examined} active={active}"
            );
        }
    }
}
