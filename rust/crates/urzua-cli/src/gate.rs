pub(crate) use urzua_core::rules::gate_and_run as gated;

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
