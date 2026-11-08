---
Stable-Id: 01M2MF5RYBQG72GMM6FE709HEP
Status: Fixed
Found-in: 'building a synthetic corpus to reproduce BUG-23 -- every finding came back `line: 1` while the H1 sat on line 7'
Realized-by: code:rust/crates/urzua-core/src/rules/filename.rs#filename_title_consistency@sha256:f6681398b562397d5afeb0c8e01faa51b58cfd29fa63ef73cd666892ce3acfbb, code:rust/crates/urzua-core/src/rules/filename.rs#first_h1@sha256:41b9f17545b8441d83451f6d2bce19132ac431ad02baf3aa9e07371eb2cb2f64, test:rust/crates/urzua-core/src/rules/filename.rs#filename_title_consistency_reports_the_real_h1_line_observed_failing@sha256:e48108cb7d7bb8f39f1216b0da3194be295ebcee8ddc7090785028b603da4764
---
# 29 — filename.title-consistency hardcodes line 1, so every finding points past the frontmatter

## What was wrong

Both `Finding` sites in `filename_title_consistency` wrote `line: Some(1)` as a literal. Every record
in this corpus opens with YAML frontmatter, so an H1 is never on line 1 -- the real
`docs/adr/ADR-36-filenames-carry-the-type-prefix-going-forward.md` carries its H1 on line 11.
Every finding this rule emitted pointed an editor at the `---` opening the header instead of the
title it was complaining about.

The information was available and discarded: `title_number` located the heading with
`.lines().find(..)` and kept only the digits, dropping the index.

Reproduced against a three-record synthetic corpus before the fix — all three findings reported
`line: 1` against H1s on line 7.

## Why nothing caught it

No test asserted the `line` field of a `filename.title-consistency` finding; the rule's existing
tests assert on message text only. And this corpus produces **zero** findings from this rule — every
record's filename and H1 agree — so the wrong value was never rendered anywhere a reader would see
it. A field that is only wrong on a path nothing exercises is invisible to both the test suite and
review.

## The fix

`first_h1` returns the line it matched, and both findings carry it. The no-H1 case carries `line:
None` rather than a fabricated position, since there is no title to point at.

## References

- `rust/crates/urzua-core/src/rules.rs` -- `first_h1` and `filename_title_consistency`.
- BUG-23 -- fixed in the same change; this defect was found while reproducing it.
- BUG-30 -- the other defect in the same function, same change.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Filed and fixed in the same change. **Why:** found by reproducing `BUG-23` against a synthetic corpus rather than reasoning about the code -- the wrong line number is invisible in this repo, where the rule never fires, and only appears once a corpus actually trips it. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 0 file-level, 2 scoped. Evidence: rust/crates/urzua-core/src/rules/filename.rs#filename_title_consistency@sha256:f6681398b562, rust/crates/urzua-core/src/rules/filename.rs#first_h1@sha256:41b9f17545b8 Reason: Pinned the locators re-pointed in this review of PR 433 against the code as it is now: record_id in rules/shared.rs still accepts any non-empty numeric prefix, values.rs normalize still strips leading zeros so ADR-0034 and ADR-34 are one id (BUG-2), filename_title_consistency in rules/filename.rs still separates an unnumbered H1 from an absent one and reports the H1's own line (BUG-23, BUG-29), first_h1 in rules/filename.rs still ignores an H1 inside a fenced block (BUG-30), discovery.rs scope_to_requested_paths still narrows check to the requested paths (BUG-1), and doctor.rs DoctorReport is still the JSON doctor emits (BUG-4). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | `Realized-by` re-pointed at where the evidence now lives: `code:` `rules.rs` replaced by `rules/filename.rs#filename_title_consistency` and `rules/filename.rs#first_h1`, each checked against the code it names. **Why:** review of PR 433: the fix is intact but moved out of `rules.rs`. `first_h1` still returns the title with its own 1-indexed line past the frontmatter and `filename_title_consistency` reports that line, not line 1; the regression test was already pinned and still resolves. | **substantive** |
