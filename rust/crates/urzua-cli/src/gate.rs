use urzua_core::config::Config;
use urzua_core::report::{Finding, RuleExecution};

/// A rule a repository has not declared does not run at all -- it is not run
/// and then filtered, because an opt-in rule that still costs its own
/// execution is opt-in in name only. The declared `level` replaces whatever
/// severity the rule body chose, which is the whole point of MILE-80: the
/// severity is the repository's call, not `rules.rs`'s.
///
/// Shared rather than owned by `check`. `audit` ran two rules directly and so
/// could not be declined by any configuration (BUG-42); a third command
/// reading rules directly would reintroduce that, and a second copy of this
/// function would drift from the first.
pub(crate) fn gated(
    config: &Config,
    id: &str,
    run: impl FnOnce() -> (RuleExecution, Vec<Finding>),
) -> (RuleExecution, Vec<Finding>) {
    use urzua_core::config::RuleLevel;
    use urzua_core::report::{FindingSeverity, RuleStatus};

    match config.rules.get(id).map(|s| s.level) {
        None | Some(RuleLevel::Off) => (
            RuleExecution {
                rule: id.to_string(),
                population: None,
                records_examined: 0,
                scope: urzua_core::report::RuleScope::Records,
                status: RuleStatus::NotEnabled,
                examined_records: Vec::new(),
            },
            Vec::new(),
        ),
        Some(level) => {
            let severity = match level {
                RuleLevel::Error => FindingSeverity::Error,
                RuleLevel::Warn => FindingSeverity::Warning,
                RuleLevel::Off => unreachable!("handled above"),
            };
            let (exec, mut findings) = run();
            for f in &mut findings {
                f.severity = severity;
            }
            (exec, findings)
        }
    }
}

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
