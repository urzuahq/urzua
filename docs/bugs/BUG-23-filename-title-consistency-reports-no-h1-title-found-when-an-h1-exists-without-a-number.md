---
Stable-Id: 01M28H5J5BV7WN2ER002C7M37A
Status: Fixed
Found-in: 'migrated from GitHub issue #1 (Phase 8.1, filed 2026-09-05), which ran `urzua check` against foreign ADR corpora -- npryce/adr-tools and upstream adr/madr -- and reported a uniform "no H1 title found" against records whose H1 was plainly present. See the correction below: only one of the two cited shapes can still reach that branch.'
Regression-test: 'rust/crates/urzua-core/src/rules.rs :: filename_title_consistency_distinguishes_an_unnumbered_h1_observed_failing, filename_title_consistency_keeps_the_absent_h1_message, filename_title_consistency_treats_an_empty_h1_as_absent'
Realized-by: code:rust/crates/urzua-core/src/rules.rs
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
