---
"urzua": major
---

Every rule is opt-in and carries a declared severity (`MILE-80`, implementing `ADR-53`).

`.urzua/config.yaml` gains a `rules` table. A rule that is not named there does
not run, and appears in `rules_executed` as `not-enabled` rather than being
omitted -- "off" and "ran clean" stay distinguishable. The declared `level`
(`off`/`warn`/`error`) replaces whatever severity the rule body chose: severity
was 34 hardcoded literals with no config key, despite `SPEC-1` listing it as a
v0 configuration surface.

`schema_version` is now 2. An older config fails with "schema_version 2 not
supported" rather than an unknown-field error.

**Breaking.** A config with no `rules` table runs no rules. `urzua init` writes
every rule at `warn`, so an adopted corpus is told what is irregular without
being blocked on day one.

`pointer.resolution` is split. It emitted a finding for every reference that
*resolved* -- 172 of 213 findings on this project's own corpus were the tool
announcing a reference worked. Resolution success now reports nothing; the
target's status is `pointer.target-status`, which fires only on statuses a
repository declares via `not_in`. One rule id cannot carry two severities, so
these could never be levelled apart while they shared one.

A rule name this build does not ship, or an option on a rule that does not take
it, is a load-time error naming the valid alternatives.

`field.quality` is split the same way (`BUG-38`). `Blank` and `Placeholder`
mean a required field was forgotten; `Pending` means someone declared the work
unfinished. Sharing one rule id left no correct setting -- `error` blocked CI on
a deliberate marker, `warn` stopped a genuinely empty field from blocking.
`field.pending` is now its own opt-in rule.
