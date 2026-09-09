---
Stable-Id: 01M23VWTEK4ZKNMZPX4VW70D2P
Status: Fixed
Found-in: 'audited live, while reviewing the SPEC-19 reframe (from "AGENTS.md instructions" to "repository agent guardrails") -- checking whether `Parent` should also be corrected on SPEC-19 surfaced the same question for every other spec'
Regression-test: 'none in the code sense -- this is a corpus-content defect, not a rule/behavior defect. `pointer.resolution` only ever verified that `Parent` resolves to an existing record (ADR-40), never that it''s the *right* record; no rule can check that mechanically. Verified by hand: each corrected value now matches that spec''s own real `Implements`/`Derives-from` lineage instead of a blanket default.'
---
# 10 — Parent field defaults to SPEC-1 across most specs without a real relationship

## What was wrong

Of this corpus's 18 real specs, 17 carry a `Parent` field at all (`SPEC-1` itself, the root, never
has one) -- and every single one of those 17 pointed to `SPEC-1`, with zero exceptions and zero
differentiation ever exercised. For 10 of them that's genuinely true: `SPEC-2`–`5`, `8`, `11`–`15`
are the CLI-subcommand specs `SPEC-1`'s own Purpose section explicitly names as split out of it
("this spec is the parent of a sequential set"), and lists by title in its own table.

For the other seven -- `SPEC-6`/`9`/`10` (the `milestone`/`bug`/`waiver` record-type specs),
`SPEC-16`/`17`/`18` (the `adr`/`rfc`/`spec` schema specs), and `SPEC-19` (`AGENTS.md`) -- it's false.
`SPEC-1`'s own child-spec table doesn't mention any of these seven. Each one's real lineage is
already fully expressed by its own `Implements` pointer (`ADR-34`, `ADR-35`, `ADR-11`, `ADR-10`
(three times), `MILE-36`) -- none of which cite `SPEC-1`'s actual content (a v0 CLI's scope, success
criteria, and command set) as their reasoning. `SPEC-9`'s own Purpose section even calls itself a
"sibling" of `SPEC-6`/`SPEC-10`, a peer relationship, while its header simultaneously claimed
`SPEC-1` as a parent -- two different and inconsistent relationships stated about the same document.

## Why nothing caught it

`ADR-40` ("Parent resolves the same as Implements/Derives-from") only decided that `Parent` gets
*validated for existence* like the other two pointer fields -- it explicitly never defined what
"parent" is supposed to *mean*, and says so directly. `SPEC-18` (the `spec` type's own schema spec)
declared `Parent` as a known field because "every spec already carries it," not from a designed
semantic contract. With no rule checking (or able to check) whether a `Parent` value expresses a
*real* relationship rather than just *a* resolvable one, a wrong default could ride along
indefinitely without ever tripping a check.

**The live code path doesn't force this default.** Checked directly: `run_new` (`main.rs`), for a
`yaml-frontmatter`-shaped type like `spec`, builds the header via `render_synthetic_yaml` (fills only
`required_fields` -- `Parent` was never one) and discards `.urzua/templates/spec.md`'s own throwaway
header entirely, keeping only its body from the first `## ` heading on (`template_body`). So
`urzua new spec` never actually emits a `Parent` value at all -- it isn't an engine requirement.

The real mechanism was `.urzua/templates/spec.md` itself: its discarded-but-still-human-visible
header hardcoded `Parent: SPEC-1 (v0 CLI).` as literal boilerplate -- carrying `BUG-7`'s exact
`(v0 CLI).`-annotation anti-pattern in the process. Anyone (human or agent) opening that file to see
"what a new spec's header should look like" when hand-authoring one (every spec in this corpus
predates `urzua new spec` actually working, per `MILE-74`) would reasonably copy it, with no way to
know from reading the template that this particular line was never executed. A stale example taught
the wrong lesson for as long as it sat there unexamined.

## Fix

Corrected `Parent` to `—` on the seven specs named above; kept `Parent: SPEC-1` only where
`SPEC-1`'s own prose documents a real split-off relationship. Fixed the template itself -- removed
the hardcoded `Parent: SPEC-1 (v0 CLI).` boilerplate, replaced with an instructional comment (`Parent
only if a real split-off relationship exists; otherwise omit`) so a future author copying the
template's shape by eye no longer inherits a false default. Landed alongside `MILE-91`, which adds a
new required `Subject` field to `spec` and updates `SPEC-18` to state `Parent`'s real, narrower
meaning going forward: a genuine narrowing/split-off relationship to a broader spec, not a default
root pointer.

## References

- MILE-91 -- the milestone that carries this fix (and the unrelated but co-landed `Subject` field
  addition, backfilled in the same pass since both touch every spec's header).
- ADR-40 -- the decision this bug's "why nothing caught it" section traces the gap to: `Parent`
  validated for existence, never for meaning.
- SPEC-18 -- the `spec` type's own schema spec, updated by `MILE-91` to state `Parent`'s intended
  meaning explicitly.
- SPEC-1 -- its own child-spec table is the source of truth this bug's corrected values were checked
  against.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-09 | Initial bug record, `Status: Fixed`. **Why:** found live auditing every spec's `Parent` value against `SPEC-1`'s own child-spec table and each spec's real `Implements`/`Derives-from` lineage -- 7 of 18 specs claimed a relationship `SPEC-1` itself never documented. | **substantive** |
