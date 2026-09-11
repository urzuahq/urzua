---
Stable-Id: 01M28H5J5BV7WN2ER002C7M37A
Status: Open
Found-in: 'migrated from GitHub issue #1 (Phase 8.1, filed 2026-09-05), which ran `urzua check` against three real foreign ADR corpora -- npryce/adr-tools and upstream adr/madr -- and hit a 100% error rate on `filename.title-consistency`; every record reported "no H1 title found" despite an H1 being plainly present (`# 1. Record architecture decisions`, `# Add Status Field`). Re-verified live in this repo on 2026-09-11: the single uniform message is still the only one emitted.'
Regression-test: 'not yet written -- needs a planted-violation test per message branch (an H1 that is genuinely absent vs. an H1 present but carrying no parseable number), observed failing on the current single-message code first'
Implements: —
---
# 23 — filename.title-consistency reports "no H1 title found" when an H1 exists without a number

## What was wrong

`filename_title_consistency` (`rust/crates/urzua-core/src/rules.rs:905-914`) branches on
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

## References

- `rust/crates/urzua-core/src/rules.rs` -- `filename_title_consistency` and `title_number`, the
  single-message branch this bug is about.
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
