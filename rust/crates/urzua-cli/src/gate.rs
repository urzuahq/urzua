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
    use urzua_core::report::{RuleScope, RuleStatus};
    executed
        .iter()
        .any(|e| e.status == RuleStatus::Ran && e.scope == RuleScope::Records)
}
