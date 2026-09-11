---
Stable-Id: 01M26DVVGZAATR67X7Z5P637ZW
Status: Open
Found-in: 'noticed live comparing urzua new''s auto-filled Author value (resolve_identity''s bare login, e.g. `beauwilliams`) against the hand-typed corpus convention (`''@beauwilliams''`, quoted) -- asked why the tool''s own output doesn''t match, and found the `@` serves no function anywhere in the codebase, while forcing the exact YAML-quoting workaround its own presence necessitates'
Regression-test: 'none in the code sense -- this is a corpus-content defect, not a rule/behavior defect, the same class as BUG-10. No code changed: `resolve_identity()` already emitted the bare login and `render_synthetic_yaml` already serializes through a real YAML mapping, so no rule could regress here. Verified instead by parsing every record header in the corpus through `yaml.safe_load` after the backfill -- all 89 `Author` values parse as unprefixed plain scalars, zero parse failures -- and by confirming directly that `Author: @beauwilliams` unquoted is still a `ScannerError` while `Author: beauwilliams` is not. The form is now stated in SPEC-16 rather than left as an undocumented hand-copied habit.'
---
# 18 — the '@' prefix on Author values is purely decorative and breaks unquoted YAML

## What was wrong

Every hand-written `Author` value in this corpus is quoted (`'@beauwilliams'`), while
`resolve_identity()` (used by both `urzua new` and `fix --apply`) returns the bare login with no
`@` at all. Checked whether the `@` serves any mechanical purpose before assuming either side was
"right": grepped every crate for code that reads, strips, or validates an `@`-prefixed identity --
nothing. The prefix is purely decorative, copied forward by hand from record to record.

Worse, it's actively YAML-hostile. Confirmed directly (not just from the spec): a plain scalar
starting with `@` is invalid, unquoted YAML --

```
$ python3 -c "import yaml; yaml.safe_load('Author: @beauwilliams')"
ERROR: found character '@' that cannot start any token
```

`@` is a reserved YAML indicator character, the same class of problem `BUG-12` found for a
backtick-starting `Subject` value. Every hand-typed `'@name'` Author value only needs quoting
*because* the `@` is there -- remove it, and the value is a perfectly ordinary plain scalar,
matching what the tool's own auto-fill already produces unquoted, correctly, today.

## Why nothing caught it

Nothing about a correctly-quoted `'@beauwilliams'` value ever failed to parse -- the quoting was
always applied by hand alongside the `@`, so the two together never actually broke anything in
practice. The gap only surfaces by asking "does this prefix do anything," which nobody had reason to
ask until comparing the tool's own auto-filled output against the hand-typed convention side by side
and finding they disagreed.

## References

- `rust/crates/urzua-io/src/lib.rs` -- `resolve_identity`, whose bare-login output is already
  correct and needs no change.
- BUG-12 -- the same defect class (a reserved YAML indicator character forcing avoidable quoting),
  found for `Subject` instead of `Author`.
- The `Author: '@...'` lines this corpus carried when this bug was filed -- the convention it
  questions. Since corrected; none remain (see the revision log below).
- The `Deciders: '@...'` lines this corpus carried alongside them -- the same defect in the same
  field family, found while fixing this one and corrected in the same pass. MILE-78 had already
  treated `Author`/`Deciders` as one unit when it backfilled both.
- `docs/specs/SPEC-16-the-adr-record-type.md`, `SPEC-17`, `SPEC-18` -- where the no-`@` form is now
  stated for each type that requires `Author`. SPEC-17/18's "same as `adr`" pointers scoped only to
  the resolution mechanism, never the written form, so each states it directly rather than relying
  on an inheritance that wasn't actually there.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-10 | Initial bug record, `Status: Open`. Not yet fixed -- whether to drop `@` going forward only (existing records untouched, both forms remain valid once quoted) or backfill every existing record to match is not yet decided. | **structural** |
> | 2026-09-11 | Backfilled all 132 `Author`/`Deciders` values across 88 files to the bare login, and stated the form in SPEC-16 (bumped `0.6` -> `0.7`). **Why:** of the two scopes this record left open, backfilling is the only one that actually removes the hazard -- dropping `@` going forward alone would have left 132 quoted values in place and the corpus carrying two conventions indefinitely. Scope note: extended to `Deciders`, which this record's own text didn't name. It is the identical defect in the same field family (44 `'@beauwilliams'` values, same forced quoting), and fixing only `Author` would have left a single record showing `Author: beauwilliams` beside `Deciders: '@beauwilliams'` -- a new inconsistency rather than a fix. `Status` left `Open` for the user to move. | **substantive** |
> | 2026-09-11 | Corrections to the entry above, from an adversarial review of the change. (1) SPEC-16 had been given the wording "written exactly as `resolve_identity()` emits it -- a bare login"; that is wrong. Only the `gh api user` tier emits a login -- `git config user.name` emits a display name and `--by` arbitrary free text, and `render_synthetic_yaml` legitimately quotes a value containing a colon or comma. Reworded to claim only what this bug actually establishes: no decorative `@` prefix. The same overclaim is left standing in this record's own "What was wrong" narrative above, which is a point-in-time account, not a live assertion. (2) This record's References cited sets the same change had emptied; reworded to past tense. (3) SPEC-17/18 were said to inherit the form via their "same as `adr`" pointers -- those scope to the resolution mechanism only, so both now state the form directly. (4) The corpus held 134 such values, not 132: ADR-37's two were already unprefixed, so this record's "every hand-written value" framing was never exactly true. | **substantive** |
