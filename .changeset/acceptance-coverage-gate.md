---
default: patch
closes: []
---

The corpus acceptance report is now a gate. An unfiltered acceptance run fails when a rule has no
failing case and is not on the committed allow-list, `tests/acceptance/coverage-allowlist.yaml`,
where each entry carries a rule id and a reason. The list can only shrink: an entry for a rule
outside the fixed ceiling is refused, and an entry whose rule gained a case or left the rule table
is stale. Five rules are on it; `narrative-field.stale` and `waiver.matches-nothing` gained failing
cases.
