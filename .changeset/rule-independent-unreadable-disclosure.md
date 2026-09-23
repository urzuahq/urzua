---
default: patch
---

Fixes `BUG-125`: a rule's silence about `Outcome::Unreadable` (a header present but not parseable)
used to depend on `header.required-fields` being separately enabled to surface it — one opt-in rule's
honesty depending on a second, independently opt-in rule, violating "rules should not depend on other
rules." `Population` now discloses its own `unreadable()` count directly, and the fix lives in the
shared `census`/`census_records` machinery, so it covers every current and future rule that reads
`record.header` with no exceptions (verified programmatically against every such rule in `rules.rs`).

`RFC-45`/`ADR-63`/`MILE-111` also adds `relation.target-status-undeclared`: a resolved pointer or
narrative reference whose target type never declares `Status` is now flagged by one dedicated rule
(matching `header.field-case-mismatch`'s precedent), rather than `pointer.target-status` and
`narrative-field.stale` each inventing its own version of the same check. `claim.status-agreement`
resolves claim-sourced references, a different source the new rule does not cover; it still silently
skips a target whose type doesn't declare `Status`, unchanged and out of scope for this round.

Fixes `BUG-126`: `type.record-outside-declared-dir` no longer warns about a record staged for
deletion that sits below (not directly in) a declared dir.

`BUG-127` (a suspected symlink-argv defect in `resolve_argv_overrides`) was investigated and re-graded
`Not a bug`: `relative_scopes` already canonicalizes — and thus dereferences — every argv path before
`resolve_argv_overrides` runs, so the reported defect is not reachable through the real CLI call
chain. No behavior change; the vacuous regression test for it is removed.

Also enables `config.header-none-has-no-required-fields`, an already-built rule that was never turned
on in this repository's own config, found via an audit comparing `ALL_RULES` against the enabled
`rules:` block.

No adopter-facing behavior change to existing findings: verified with the full test suite, clippy,
`make ci`, and a real-corpus `check` run.
