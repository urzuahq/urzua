---
Stable-Id: 01M36CDVC74HV4JF98QN78ZR5Q
Status: Fixed
Found-in: "Investigating RFC-45's scope; the user asked whether Absent/Unreadable being silently folded together was itself a design problem"
Regression-test: "field_quality_discloses_unreadable_on_its_own_observed_failing, rules.rs"
---
# 125 — a field rule's silence about an unreadable header depends on a separate opt-in rule being enabled to disclose it

## What was wrong

`field.quality`'s own doc comment justifies folding `Unreadable` into "not examined" with no separate
disclosure: *"`header.required-fields` reports the parse error itself, once per record rather than
once per slot."* That reasoning assumes `header.required-fields` is enabled. `ADR-53` makes every
rule an independently declared, opt-in policy — nothing requires two rules to be enabled together,
and no mechanism enforces it.

Reproduced live, on `main`, in a disposable scratch repo:

```yaml
schema_version: 2
rules: {field.quality: error}
record_types:
  adr: { dir: "docs/adr", required_fields: ["Status"] }
```

A record whose header does not parse at all:

```text
$ urzua check
{"status": "ok", "files_examined": 1, "records_read_by_any_rule": 0, "findings": [], ...}
```

Clean exit, zero findings, on a corpus containing a genuinely unparseable record. Enabling
`config.scope-matches-nothing` alongside `field.quality` does not catch it either — that rule only
flags a population whose `out_of_scope` count is non-zero; an unreadable header reports
`out_of_scope: 0` (the record is structurally in scope, just unreadable), so it passes silently
through that gate too. This is `ADR-55`'s exact defect class — *"a check that reports success without
looking"* — reachable through an entirely ordinary, single-rule config.

## Why nothing caught it

`field.quality`'s reasoning was correct for *this repository's own config*, which has always enabled
`header.required-fields` alongside every field-level rule. Nothing checked whether that pairing is
actually required by the code, only assumed by convention.

Verified after the fix, not just assumed: every rule function in `rules.rs` that touches
`record.header` at all also routes through `census`/`census_records` (checked programmatically, zero
exceptions). Fixing the shared machinery once — rather than `field.quality` individually — closes this
for `field.pending`, `field.untrimmed-value`, `header.field-case-mismatch`,
`header.pointer-field-clean`, `narrative-field.stale`, and every other current and future rule built
the same way, with no per-rule migration needed.

## The design principle this violates

Stated directly, per the user: **no rule may depend on another rule being enabled to cover a gap in
its own disclosure.** Each rule's `Population` must be self-sufficient — correct and honest on its
own, under any subset of `ALL_RULES` a repository chooses to enable, since `ADR-53` guarantees no
particular subset. A rule whose correctness assumes a sibling rule's config is an undeclared,
unenforced coupling between two things `ADR-53` says are independent.

## What changed

`census`/`census_records` (the shared machinery every field/record-level rule already builds its
`Population` through) now tally `Outcome::Unreadable` into a new, disclosed `Population.unreadable()`
count, alongside `eligible`/`examined`/`out_of_scope`. Fixed once, in `report.rs`, not per rule.

## References

- `RFC-45` — where this bug's fix is designed and tracked.
- `ADR-53` — "declared, not voted"; the principle this bug's root cause violates by assuming a pairing
  `ADR-53` never guarantees.
- `ADR-55` — the defect class this bug is a live instance of.
- `MILE-106` — built a narrower version of this gate (`config.scope-matches-nothing`, checking only
  `out_of_scope`), which does not catch this shape.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed. **Why:** the user asked whether silently folding `Absent`/`Unreadable` together was itself a design problem; investigating surfaced a live, reproducible instance of `ADR-55`'s core defect class reachable through an ordinary single-rule config, not a hypothetical. | **substantive** |
> | 2026-09-23 | Fixed via `RFC-45`/`ADR-63`: `unreadable` disclosed by `census`/`census_records` structurally. Verified against the exact scratch reproduction above, and verified programmatically that every header-reading rule routes through the fixed machinery. | **substantive** |
