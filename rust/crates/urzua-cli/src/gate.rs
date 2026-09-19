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
                records_examined: 0,
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
