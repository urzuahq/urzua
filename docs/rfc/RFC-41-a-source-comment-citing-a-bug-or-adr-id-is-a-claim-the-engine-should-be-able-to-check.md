---
Stable-Id: 01M33WWP42G5E16ZC2F78KP1JR
Status: Draft
Date: 2026-09-22
Author: beauwilliams
---
# 41 — a source comment citing a bug or ADR id is a claim the engine should be able to check

## Summary

`embodiment.locator-exists` already checks the direction "a record's `Realized-by` locator names a
real file." This proposes the reverse: a source comment citing `BUG-104`, `ADR-57`, etc. is itself a
claim that record exists, and nothing checks it. A rule doing so would have caught, mechanically, a
defect this project's own review process just found by a manual `grep -rn "BUG-104"` sweep.

## Motivation

During round 15's review-finding fixes, two source comments in `record.rs` and `init.rs` cited
`BUG-104` — a bug record that was created, then deleted after its finding was directly verified false,
with the real defect refiled as `BUG-108`. The two comments, written before the deletion, were never
updated. Nothing caught this until a human-initiated `grep` swept the tree by hand, after a
`/code-review` pass separately flagged one instance as a "temporal language" violation and the fix
happened to notice the other while correcting it.

This is exactly the shape `embodiment.locator-exists` was built to prevent in the other direction: a
citation that silently outlives the thing it points at. The asymmetry is not principled — a locator
naming a nonexistent file and a comment naming a nonexistent bug are the same failure, discovered the
same way (a human happens to notice), just pointed in opposite directions.

## Proposal

A new rule, `code.dangling-bug-reference`, scanning source files (extensions declared in config, not
hardcoded — this project's own `ADR-53` forbids the engine assuming what a repository's source
languages are) for a citation pattern (`BUG-\d+`, `ADR-\d+`, `RFC-\d+`, etc., matching this project's
own record-type prefixes) inside comments, and reporting one whose cited id has no corresponding record
under the configured `dir` for that type.

This is structurally different from every existing rule in `rules.rs`: it is the first rule reading
*source code* rather than record files. `urzua-core`'s purity boundary (`ADR-5`: no I/O in the rule
crate) is unaffected — the caller (`urzua-cli`) already reads file contents and hands them to rules
exactly as it does for record bodies; this only widens which tracked files get read and handed in, not
who does the reading.

## Open questions

- **Which comment syntaxes to recognize**, and whether that list is itself declared config (a
  `code_comment_prefixes: ["//", "#", "--"]`-shaped key per this project's own "declared, not voted"
  principle, `ADR-53`) or a fixed set covering common languages. Given this project's own prior
  experience overfitting a vocabulary to two corpora (`ADR-53`'s "left open" clause on `RFC-33`'s
  function-name grid), a third real corpus using an uncommon comment syntax is the cheap gate before
  freezing this list.
- **Whether a citation inside a string literal or a doc example counts.** This proposal's own
  motivating instance is a doc comment (`///`), which is unambiguous; a citation embedded in a code
  string (e.g., an error message quoting a bug id) is a different, weaker claim and may need excluding
  explicitly rather than silently mis-flagged.
- **Severity and false-positive risk on renamed/renumbered records.** A record can legitimately be
  superseded (`Supersedes / Superseded-by`) without the citing comment being wrong — only *deletion*
  (this RFC's motivating case) makes a citation dangling. The rule needs to check existence, not
  currency, to avoid flagging every comment citing a superseded-but-still-real record.
- **Scope of "source"**: does this rule also scan `.md` prose outside the declared record types (e.g.
  `README.md`, this repository's own `docs/` files that are not themselves records), where the same
  staleness risk exists but the file isn't one this engine already parses as a record?

## Non-goals

- **Not a general link-checker.** This is scoped to this project's own record-id citation convention
  (`TYPE-NUMBER`), not arbitrary URLs or cross-references.
- **Does not fix `embodiment.locator-exists`'s own scope** — that rule is unaffected; this proposes its
  mirror, not a change to it.
- **Not implemented here.** Filed for later scoping and a milestone, per the instruction that raised it.

## References

- `embodiment.locator-exists` (`ADR-18`/`ADR-32`) — the existing rule checking the reverse direction
  (a record's locator naming a real file).
- `ADR-53` — "declared, not voted" governs the open question on comment-syntax/file-extension scope.
- The round-15 branch's own `BUG-104`→`BUG-108` renumbering — the concrete incident that raised this.
