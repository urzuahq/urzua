---
Stable-Id: 01M36PD918XPGFGAPRCK161BE5
Status: Open
Found-in: "A /code-review v0.3.0...main pass, round 23"
Regression-test: "not yet written -- Status: Open, no fix decided yet"
---
# 131 — the identity index's silent first-seen-wins resolution has no disclosure when identity.collision is disabled

## What was wrong

`build_index_reporting_collisions` builds the shared `RecordIndex` every reference-resolving rule
reads, keeping only the first-inserted record for a colliding identifier
(`index.entry(key).or_insert(record)`) -- unconditionally, every run. Six independently opt-in rules
share this one index: `pointer.resolution`, `pointer.target-status`, `claim.status-agreement`,
`relation.supersession-reciprocity`, `relation.target-status-undeclared`, `narrative-field.stale`.

If a repository enables any of those six but leaves `identity.collision` off -- each rule is gated
independently, per `ADR-53` -- a real identifier collision produces no finding at all, and every one
of the six silently resolves against whichever record happened to be inserted first. `round-22-fixes`
made that "first" record deterministic (`Config::sorted_type_names`), but deterministic-and-wrong is
still wrong: `claim.status-agreement` could judge a claim against the wrong one of two same-ID records
and report clean.

## Why this may need a design decision, not a mechanical fix

This isn't quite `BUG-125`'s shape (a rule's own body misclassifying `Unreadable` as `Absent`) -- it's
a shared, always-built index silently picking a winner for consumers that never asked to be told about
the ambiguity. Plausible shapes for a fix, each with a real tradeoff:

- Disclose every identity collision unconditionally as a `Notice` (the existing lower-severity,
  always-visible channel `check.rs` already uses for e.g. an absent `claim_paths` entry), regardless of
  whether `identity.collision` is enabled to turn it into a blocking `Finding`. Closest in shape to
  `RFC-45`'s "disclosure independent of a sibling opt-in rule" fix for `BUG-125`.
- Make `identity.collision` (or the disclosure alone) effectively mandatory infrastructure rather than
  opt-in, since the other six rules' own correctness quietly depends on it -- a real change to `ADR-53`
  "declared, not voted" for this one rule specifically, needing its own justification.
- Have each of the six consuming rules check `RecordIndex` for ambiguity at the point of resolution and
  degrade to an explicit unresolved/ambiguous state rather than silently picking the index's winner --
  correct but repeats the same check six times, the "every caller invents its own version" shape this
  project has already rejected once for `RFC-45` Part 1.

## References

- `BUG-125`/`RFC-45`/`ADR-63` -- the precedent for disclosure that doesn't depend on a sibling opt-in
  rule, the closest existing shape to a likely fix here.
- `ADR-53` -- "declared, not voted"; the tension this bug sits in, since the *fix* may mean treating one
  rule's data-integrity role differently from every other opt-in rule.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed. **Why:** found by a full-release code review; verified the index is built unconditionally and shared by six independently-gated rules with no disclosure path when `identity.collision` itself is off. Left `Status: Open` -- the right fix shape is a real design question, not a one-line change. | **substantive** |
