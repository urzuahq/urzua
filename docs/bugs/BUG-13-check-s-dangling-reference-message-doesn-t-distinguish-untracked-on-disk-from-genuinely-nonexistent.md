---
Stable-Id: 01M260GGTATCK7PSKRQY2K7T0M
Status: Open
Found-in: 'hit live drafting ADR-45 while SPEC-20 existed on disk but was not yet `git add`ed -- ADR-45''s own Implements/References pointer to SPEC-20 reported `relation.supersession-reciprocity`/`pointer.resolution` findings identical in wording to a reference to a record that does not exist anywhere, even though the file was sitting right there in the working tree'
Regression-test: 'not yet written -- fix scope not yet decided (see Why nothing caught it: the distinction needs filesystem access urzua-core is deliberately pure of, per the same purity boundary ADR-5/6 established)'
---
# 13 — check's dangling-reference message doesn't distinguish untracked-on-disk from genuinely nonexistent

## What was wrong

`urzua check` (correctly, by design -- ADR-6's git-tracked-only discovery) only examines git-tracked
and staged files, never a raw filesystem walk. A reference to a record that exists on disk but isn't
yet staged reports the exact same message as a reference to a record that doesn't exist anywhere:
`"<field>: <reference> does not resolve to any discovered record"` /
`"does not resolve to any discovered record"`. A person (or agent) actively drafting two
cross-referencing records has no way to tell, from the message alone, whether they made a real typo
or just forgot `git add` on the target -- both read identically as "this doesn't exist."

## Why nothing caught it

No test exercises this specific distinction -- every existing `pointer_resolution`/
`supersession_reciprocity` test constructs its fixture records directly in memory (never touching a
real filesystem or git index at all), so there was never a scenario where "discovered" and "exists
on disk" could actually diverge in a test. The gap is architectural, not just a missing test case:
`urzua-core`'s rule functions are deliberately pure, no filesystem or git access (the same purity
boundary `ADR-5`/`ADR-6` enforce, checked by `purity.rs`'s own compile-time dependency guard) --
they only ever see the `records` slice `urzua-cli`'s discovery layer hands them. Distinguishing
"genuinely doesn't exist" from "exists on disk but wasn't discovered" needs information (a real
filesystem stat, or knowledge of what git considers untracked) that the pure rule layer structurally
cannot have. A fix has to live at the `urzua-cli` boundary -- either enriching a dangling finding's
message after the fact by checking whether any candidate path matching the reference exists on disk,
or making `discover_tracked_files` itself distinguish "not tracked" from "doesn't exist" and passing
that through -- not a change to any pure rule function.

## References

- `rust/crates/urzua-core/src/rules.rs` -- `pointer_resolution`, `supersession_reciprocity`, whose
  dangling-reference messages this bug is about.
- `rust/crates/urzua-io/src/lib.rs` -- `discover_tracked_files`, the git-tracked-only discovery
  boundary this fix would need to extend past, without violating it for the actual check itself.
- ADR-6 -- the git-tracked-only discovery decision this bug does not propose changing, only wants a
  clearer message about.
- ADR-5 -- the `urzua-core` purity boundary explaining why this can't be a pure-rule-layer fix.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-10 | Initial bug record, `Status: Open`. Not yet fixed -- exact mechanism (post-hoc message enrichment in `urzua-cli` vs. a richer discovery-layer signal threaded through) not yet decided. | **structural** |
