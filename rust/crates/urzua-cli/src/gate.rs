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

/// `Ok` means "checked and clean", so it requires that a rule capable of
/// judging records actually ran. A run with no such rule has established
/// nothing, and reporting it clean is indistinguishable from a corpus that
/// passed (ADR-55). Reachable since rules became opt-in: an empty or absent
/// `rules` table, or a table declaring only rules that read the configuration
/// or the path inventory.
///
/// Deliberately *not* `records_examined > 0`. A rule that ran and examined
/// nothing is usually correct -- a corpus with no supersessions gives
/// `relation.supersession-reciprocity` nothing to judge, and that is a clean
/// result, not an unestablished one. Requiring a non-zero count made `audit`
/// unsatisfiable on exactly the config `init` generates (BUG-84).
///
/// Telling "nothing to examine" from "could not address anything" is a real
/// distinction and a harder one; it is `MILE-106`'s subject, where
/// `type.dir-matches-nothing` is the precedent for drawing it.
pub(crate) fn any_rule_looked(executed: &[urzua_core::report::RuleExecution]) -> bool {
    use urzua_core::report::{PopulationUnit, RuleScope, RuleStatus};
    executed.iter().any(|e| {
        if e.status != RuleStatus::Ran {
            return false;
        }
        match &e.population {
            // A converted rule states what it was handed. A run in which every
            // such rule was handed nothing has established nothing, whatever
            // `records_examined` says -- the census was added this release to
            // make that visible and the gate did not read it, so a rule
            // reporting `eligible: 0` still certified the corpus `ok`.
            //
            // This is per *run*, not per rule: one rule with an empty
            // population among others that had work is the cold-start state and
            // is not a fault. `header.layout-consistency` is in it permanently
            // on this repository.
            // The unit still matters: a config or path rule with a non-empty
            // population has read declarations or filenames, not records, and
            // letting either certify the corpus is BUG-81 and BUG-83.
            Some(population) => {
                matches!(
                    population.unit(),
                    PopulationUnit::Record | PopulationUnit::Field
                ) && population.eligible() > 0
            }
            // Unconverted rules keep the old signal until every rule carries a
            // population. `audit` runs two of them, and on the config `init`
            // writes both legitimately examine nothing -- requiring a non-zero
            // count here is BUG-84.
            None => e.scope == RuleScope::Records,
        }
    })
}
