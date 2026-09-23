---
Stable-Id: 01M286MP5PYPBAS2BZ4PXEJ00C
Status: Fixed
Found-in: 'adding notices to SPEC-2''s example JSON block for ADR-46 -- the block was already wrong in three other, unrelated ways before this change touched it'
Regression-test: 'none -- a documentation correction, not a rule/behavior defect. No mechanical check verifies a spec''s example JSON against the real shipped shape (see Fix below for why that is not built here).'
---
# 21 — SPEC-2 documents an illustrative shape that doesn't match what `check` ships

## What was wrong

`docs/specs/SPEC-2-urzua-check.md`'s `## Output contract` section's JSON block is RFC-3's original,
pre-implementation illustration, never updated to match the real shipped output:

- Field names are `camelCase` (`filesExamined`, `rulesExecuted`); the real `CheckReport` serializes
  `snake_case` (`files_examined`, `rules_executed`), matching `serde`'s default rather than RFC-3's
  illustrative casing (a choice ADR-7 already made explicitly, accepting the implementation over
  re-deriving the RFC's casing).
- `status` is documented as `"ok | warn | error | not-run"` -- four values. The real `ReportStatus`
  enum has three: `ok | findings-present | not-run`. Neither the count nor two of the names match.
- The section's own prose says *"Human format on stderr; the machine channel is stdout"* -- stderr
  rendering was removed entirely by ADR-26, then narrowed further by ADR-46; there has been no
  second rendering on any stream for some time.

None of these three are related to `notices` (ADR-46's real addition, also missing from the block
until this revision) -- they're older, independent drift this revision found while touching the
section for an unrelated reason.

## Why nothing caught it

Nothing checks a spec's own documented example JSON against the real serialized output of the type
it describes -- `header.field-set-consistency` and friends validate *record* headers, not spec
*prose* against source code. SPEC-2's own 2026-09-07 revision-log entry already named this exact
class of drift once (rule names in prose not matching shipped rule ids) and fixed that instance
without a general rule following from it -- the output-contract block was simply a second instance
nobody had reason to look at until this change touched the same section for something else.

## Fix

Rewrote the `## Output contract` JSON block to the real shipped shape: `snake_case` fields, the real
3-value `ReportStatus` (`ok`/`findings-present`/`not-run`), `rules_executed` as the actual per-rule
array (`{rule, population?, status}`, not a bare count), `records_read_by_any_rule`, an accurate
`Finding` shape (no `suggestedAction`, which never existed in `Finding` at all -- a fourth inaccuracy
this record's own investigation hadn't named), and `notices`. Bumped `SPEC-2` to `0.10`.

A mechanical check that a spec's documented JSON matches its source type's real shape (the second
option this record's own text considered) is not built here -- it would need its own design for what
"matches" means against an illustrative, necessarily-truncated example, and this fix closes the
immediate defect (a wrong, stale illustration) without that larger investment.

## References

- `docs/specs/SPEC-2-urzua-check.md` -- `## Output contract`.
- `rust/crates/urzua-core/src/report.rs` -- `CheckReport`, `ReportStatus`, the real shipped shape.
- ADR-7 -- accepted `snake_case` field names over RFC-3's illustrative `camelCase`.
- ADR-26/ADR-46 -- removed the stderr rendering this section's prose still describes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-11 | Initial bug record, `Status: Open`. Not yet fixed -- whether the right fix is rewriting the block to the real shape, or a mechanical check that a spec's documented JSON matches its source type, is not yet decided. | **structural** |
> | 2026-09-23 | Fixed by rewriting the block to the real shape (the first option), decided against building the mechanical check (the second) as its own, larger investment. Found a fourth inaccuracy along the way: `suggestedAction` never existed on `Finding` at all. `Status: Fixed`. | **substantive** |
