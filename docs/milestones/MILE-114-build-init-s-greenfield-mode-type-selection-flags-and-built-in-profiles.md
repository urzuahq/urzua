---
Stable-Id: 01M378CTK581CR9DG5WXFT3N6R
Status: Planned
Phase: '1'
Track: write-path
Implements: RFC-1
---
# 114 — build init's greenfield mode, type-selection flags, and built-in profiles

## What

`urzua init` today has exactly one mode: adopt an existing corpus (`--dry-run`/`--config` are its
only flags). This milestone is the other mode `SPEC-5` has always described but never built --
a repository with no records yet, choosing which types to govern from a menu instead of having them
inferred from disk:

```
urzua init                          # interactive: pick types
urzua init --types adr,rfc,spec     # non-interactive
urzua init --types adr --dir docs   # layout root
```

Concretely: built-in profiles for `adr`/`rfc`/`spec`/`prd` (each supplying a directory, a template,
a status enum, required fields, and role requirements -- RFC-1's per-type half of core+profile, with
one fixed core); `--types`/`--dir` flags; and, since adopt's own gap is the same shape, reporting a
record-shaped file adopt could not classify by path rather than silently excluding it.

A profile is a **starting point that is written out, not a hidden default** -- `init` materializes a
profile's rules into `config.yaml` rather than referencing a built-in by name, so a user who disagrees
with a required field edits a visible line instead of overriding a rule nobody can see. Non-interactive
must be first-class from the start: `init` runs in CI, in containers, and under agents, and an
interactive-only setup path is unusable by exactly the callers this product is for.

Also folds in the two `SPEC-5` success criteria that depend on this milestone (a `urzua init --types
adr,rfc,spec` run producing a checkable config with unclassified files reported rather than errored)
and its still-open questions: whether `init` should wire CI/hooks (leaning opt-in, `--with-ci`),
where the display-number scheme gets chosen (blocked on `ADR-3`), whether adopt should infer rules or
only structure, and whether one `.urzua/` per repo holds for a monorepo with independent corpora
(`RFC-11 §6` may already answer this).

## Why

`BUG-26` found `SPEC-5` documenting this design as if built -- `urzua init --types adr,rfc,spec`
returns `error: unexpected argument '--types' found`. Closing that bug meant deciding where unbuilt
design lives once a spec is corrected to describe only what ships: a milestone, not left inline in a
spec that has to stay a complete, replayable specification of its actual current subject at every
revision (`ADR-14`). This is that relocation, not new scope -- every sentence here was already argued
in `SPEC-5` before this milestone existed.

## References

- `SPEC-5` -- `urzua init`, now `Accepted` for the adopt path this milestone's own scope excludes.
- `RFC-1` -- core+profile, which type selection instantiates.
- `BUG-26` -- found this content documented as shipped; this milestone is where it moved.
- `ADR-3` -- the unsettled identifier-encoding question behind the display-number open question.
- `RFC-11 §6` -- scope, the likely answer to the multi-corpus-per-repo open question.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed, relocating `SPEC-5`'s aspirational sections (greenfield mode, `--types`/`--dir`, built-in profiles, unclassified-file reporting) and its two unresolved success criteria and four open questions, so `SPEC-5` can be corrected to describe only what ships (`BUG-26`). | **substantive** |
