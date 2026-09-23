---
Stable-Id: 01M36PD918XPGFGAPRCK161BE5
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 23"
Regression-test: check_discloses_an_identity_collision_as_a_notice_when_the_rule_is_disabled_observed_failing, audit_discloses_an_identity_collision_as_a_notice_when_the_rule_is_disabled_observed_failing (rust/crates/urzua-cli/tests/check_integration.rs)
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

## Fix

`check` and `audit` now disclose every identity collision `build_index_reporting_collisions` finds as
an unconditional `Notice`, via the same `rules::identity_collision_notices` helper the `graph.rs` fix
introduced (extracted from that fix's own inline logic once a third call site needed it, rather than a
third copy). `identity.collision`'s own blocking `Finding` is unaffected -- exactly as opt-in as
before; the disclosure is `ADR-55` engine honesty about an ambiguity the shared index resolved, not an
`ADR-53` policy judgment about the corpus.

Two regression tests (`check`, `audit`) with `identity.collision` left disabled were verified
genuinely failing pre-fix (`notices` array absent, `Option::unwrap()` panic) and passing after.

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
> | 2026-09-23 | Reframed while working through this with the user: the earlier "the other six rules' own correctness depends on `identity.collision`" framing was imprecise and caused real confusion -- the six rules don't depend on `identity.collision` *the rule* at all; they depend on a corpus-level invariant (no two records share an ID) that `identity.collision` is merely the one mechanism that checks. The disclosure question is `ADR-55`'s (engine honesty about an ambiguity it had to resolve), not `ADR-53`'s (which rules to apply) -- an unconditional disclosure doesn't override the adopter's declared rule policy, since `identity.collision`'s own blocking `Finding` stays exactly as opt-in as before. A sibling gap found in the same investigation (`urzua graph` used the plain, non-collision-reporting index builder and disclosed nothing at all) is fixed in this same change, disclosing via `GraphReport`'s existing `notices` field. `check.rs`/`audit.rs`'s own disclosure (this bug's original scope) is still open -- `graph.rs`'s fix doesn't presuppose or complete it. | **substantive** |
> | 2026-09-23 | Fixed. `check` and `audit` now disclose collisions the same way `graph` does, via a helper extracted once three call sites needed the identical logic. Both regression tests verified genuinely failing pre-fix. | **substantive** |
