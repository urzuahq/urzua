---
Stable-Id: 01M28H5J5BV7WN2ER002C7M37A
Status: Fixed
Found-in: 'migrated from GitHub issue #1 (Phase 8.1, filed 2026-09-05), which ran `urzua check` against foreign ADR corpora -- npryce/adr-tools and upstream adr/madr -- and reported a uniform "no H1 title found" against records whose H1 was plainly present. See the correction below: only one of the two cited shapes can still reach that branch.'
Realized-by: code:rust/crates/urzua-core/src/rules/filename.rs#filename_title_consistency@sha256:448d8cd28e7e5868abee96ec6242d5d81a86ecd3d8c0c72d5101875f93dd094c, code:rust/crates/urzua-core/src/rules/filename.rs#first_h1@sha256:41b9f17545b8441d83451f6d2bce19132ac431ad02baf3aa9e07371eb2cb2f64, test:rust/crates/urzua-core/src/rules/filename.rs#filename_title_consistency_distinguishes_an_unnumbered_h1_observed_failing@sha256:2fcccbfefa84724c2a3a52bcd4972c4cede8f7301ec17acb7c9c69fdc5d97151, test:rust/crates/urzua-core/src/rules/filename.rs#filename_title_consistency_keeps_the_absent_h1_message@sha256:d0d9191840bc2bb04809129fb0c88ea19fa306d1aad5066bd3f7693c33e4e734, test:rust/crates/urzua-core/src/rules/filename.rs#filename_title_consistency_treats_an_empty_h1_as_absent@sha256:cfc25a76b0c1eba289913e8d913119b67c0c4d8a73fede10c0485ce6f095d750
---
# 23 — filename.title-consistency reports "no H1 title found" when an H1 exists without a number

## What was wrong

`filename_title_consistency` (`rust/crates/urzua-core/src/rules.rs:904-914`) branches on
`title_number(content)` returning `None` and emits one message for every reason it can return `None`:

```rust
let Some(title_number) = title_number(content) else {
    findings.push(Finding {
        ...
        message: "no H1 title found to check against the filename's number".to_string(),
    });
    continue;
};
```

`title_number` returns `None` both when the document genuinely has no H1 *and* when it has an H1
whose leading token carries no parseable number. Those are different defects with different fixes —
"add a title" versus "this corpus numbers its records somewhere other than the H1" — and a reader
gets the first message for both. Against a corpus using a different titling convention, the result
is a uniformly red run whose message is actively misleading: it says no title was found while the
title sits on line 1.

This is the same defect class as BUG-12 (a YAML header failing to parse reported as "no
header-shaped region found", discarding the real parser error) and BUG-13 (a dangling reference to
an untracked-but-present file reported identically to one that doesn't exist anywhere) — a
collapsed diagnostic where the rule knows more than it says. Third instance, which is itself worth
noting: the pattern is recurrent enough to be worth a general pass rather than three separate
one-off message fixes.

## Why nothing caught it

Every record in this repo's own corpus carries a numbered H1 matching its filename, so the `None`
branch is unreachable against corpus zero and no test has ever needed to distinguish the two causes.
`filename.title-consistency`'s existing tests exercise the mismatch branch (filename number versus
title number) and the legacy-filename skip (BUG-9), never the no-number branch. The distinction only
becomes observable against a corpus this project did not author — exactly what GitHub issue #1 did,
and why its result was worth keeping even though the header-shape half of that issue is tracked
separately.

Note this is a message-quality defect, not a false finding: `filename.title-consistency` reporting
something on a corpus whose H1s carry no number is arguably correct behaviour under ADR-36's
filename scheme. Only the wording is wrong.

## A correction to this record's own reproduction

Re-verified live on 2026-09-16 against a synthetic corpus. Of the two titles this record cites, only
one can reach the reported branch today:

- `# Add Status Field` -- first token `Add`, no digits, so `title_number` returns `None` and the
  misleading message fires. This is the defect. ✅
- `# 1. Record architecture decisions` -- first token `1.`, which yields the digit `1`. It does not
  reach the branch. And npryce files are named `0001-record-architecture-decisions.md` with no type
  prefix, so `filename_number` returns `None` and the record is **skipped entirely** by the
  `BUG-9`/`ADR-36` legacy-filename guard. Confirmed: `records_examined` counts 3 of 4 files with such
  a record present, and it produces no finding.

The claim was true when GitHub issue #1 was filed on 2026-09-05; the legacy-filename guard landed
afterwards and made half of it stale. The defect is real and unchanged — the reproduction is
narrower than recorded.

## The fix

`title_number`'s `Option<String>` is replaced by `first_h1`, which returns what it found rather than
a verdict about it:

```rust
struct H1<'a> { line: usize, text: &'a str, number: Option<u64> }
fn first_h1(content: &str, header_region: Option<(usize, usize)>) -> Option<H1<'_>>
```

Whether an H1 exists and whether it carries a number are independent questions, so they stay
independent `Option`s. Collapsing them into one absent value is what produced the single message; a
three-variant enum would have flattened the same two axes differently and kept the defect's shape.

Each cause now has its own message, the no-H1 wording unchanged so that case is not a behaviour
change for anyone.

Two further defects in the same function were found while reproducing this one and fixed in the same
change: `BUG-29` (a hardcoded `line: 1`) and `BUG-30` (an H1 matched inside a fenced code block).
`BUG-32` and `RFC-29` were filed and deliberately not fixed here.

**Severity is unchanged** — still `Error` on every branch. `ADR-36` makes flagging an unnumbered H1
arguably correct *for a corpus that declares this convention*, and there is currently no way to
declare otherwise. That gap is `RFC-29`; treating this record's fix as closing it would overstate it.

## References

- `rust/crates/urzua-core/src/rules.rs` -- `first_h1` and `filename_title_consistency`.
- BUG-12 -- the same collapsed-diagnostic class, for a YAML header's real parse error.
- BUG-13 -- the same class again, for a dangling reference that exists on disk but is untracked.
- MILE-22 -- MADR/Nygard import, which tracks the *other* half of GitHub issue #1 (declaring header
  shapes for heading-delimited and front-matter-only corpora). Deliberately not merged into this
  record: that half is a schema question, this one is a message a single rule emits.
- RFC-26 -- the proposal to stop using GitHub issues as a tracking surface, under which issue #1's
  content was migrated here.
- ADR-36 -- the filename/number scheme `filename.title-consistency` enforces.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-11 | Initial bug record, `Status: Open`. Migrated from GitHub issue #1's second half (RFC-26); the header-shape half stays with MILE-22. Not yet fixed. | **structural** |
> | 2026-09-16 | Fixed, and this record's own `Found-in` narrowed. **Why:** the fix is the smaller half -- reproducing the bug first showed that one of the two cited corpus shapes is skipped by a guard that landed after this was filed, and that the same function held two worse defects (`BUG-29`, `BUG-30`), one of which fabricates findings rather than merely wording them badly. Correcting the reproduction matters because a record marked `Fixed` on top of a stale repro cannot be re-verified by the next reader. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 3 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 0 file-level, 2 scoped. Evidence: rust/crates/urzua-core/src/rules/filename.rs#filename_title_consistency@sha256:f6681398b562, rust/crates/urzua-core/src/rules/filename.rs#first_h1@sha256:41b9f17545b8 Reason: Pinned the locators re-pointed in this review of PR 433 against the code as it is now: record_id in rules/shared.rs still accepts any non-empty numeric prefix, values.rs normalize still strips leading zeros so ADR-0034 and ADR-34 are one id (BUG-2), filename_title_consistency in rules/filename.rs still separates an unnumbered H1 from an absent one and reports the H1's own line (BUG-23, BUG-29), first_h1 in rules/filename.rs still ignores an H1 inside a fenced block (BUG-30), discovery.rs scope_to_requested_paths still narrows check to the requested paths (BUG-1), and doctor.rs DoctorReport is still the JSON doctor emits (BUG-4). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | `Realized-by` re-pointed at where the evidence now lives: `code:` `rules.rs` replaced by `rules/filename.rs#filename_title_consistency` and `rules/filename.rs#first_h1`, each checked against the code it names. **Why:** review of PR 433: the fix is intact but moved out of `rules.rs`. `filename_title_consistency` still tells an H1 with no record id (reported on its own line with the "carries no record id" message) from a record with no H1 at all (the "no H1 title found" message), and its three regression tests were already pinned and still resolve. | **substantive** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/filename.rs#filename_title_consistency` (lines 3-93) f6681398b562 -> 448d8cd28e7e. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: filename.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
