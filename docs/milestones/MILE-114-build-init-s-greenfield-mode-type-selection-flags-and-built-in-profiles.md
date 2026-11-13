---
Stable-Id: 01M378CTK581CR9DG5WXFT3N6R
Status: Done
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

## Done means

- `urzua init --types adr,rfc,spec [--dir docs]` runs non-interactively on a repository with no records. — evidence: test:rust/crates/urzua-cli/tests/init_greenfield_integration.rs#types_writes_a_config_and_templates_for_each_selected_profile@sha256:ed489550207d3c3714546944126650802ad0c443fb19c92af26543eccdc453df, test:rust/crates/urzua-cli/tests/init_greenfield_integration.rs#dir_sets_the_layout_root@sha256:fe74ff16c0b162486ba5b484af9a299e5df2b1302c33ed17d5c00de632aca89c
- An interactive `urzua init` run picks types from a menu. — evidence: test:rust/crates/urzua-cli/tests/init_greenfield_integration.rs#on_a_terminal_init_asks_which_types_to_govern@sha256:5c9dc9fe74ce49fa6bff09c91e86f5c19533a3e727a6b6d892bfa5e8402c5eea, code:rust/crates/urzua-cli/src/commands/init_greenfield.rs#prompt_selection@sha256:bd39286a93f9bb46349a30b70b8eba1a8616d43d1acb6bdaad7fb301b3e89f2a
- Built-in profiles for `adr`, `rfc`, `spec` and `prd` are materialized as visible rules in the written `config.yaml`, never referenced by name. — evidence: test:rust/crates/urzua-cli/tests/init_greenfield_integration.rs#the_profiles_are_materialized_as_visible_rules_in_the_config@sha256:8b1422bc66a9363f0c31309b3f212e2df3c8d6a9949d90bc7a51ea3d5b8ad0a4, code:rust/crates/urzua-cli/src/commands/init_greenfield.rs@sha256:bbf43c454ad071eead41247c35c6409fdacc3945db5bd7191cb64f5887fcab0c
- The `config.yaml` that greenfield `init` writes passes `check`. — evidence: test:rust/crates/urzua-cli/tests/init_greenfield_integration.rs#the_written_config_passes_check_and_new_writes_a_stable_id_for_every_type@sha256:c955fee775949944b66419eb7f31f44c5d292b958eebfd40a7d5c4bfb576038e
- Adopt reports a record-shaped file it could not classify by path instead of excluding it silently. — evidence: test:rust/crates/urzua-cli/tests/init_greenfield_integration.rs#adopt_reports_a_root_level_record_it_could_not_classify@sha256:26650cd82a71054e44a96f88d2781f4a22d4c26864f86bf53451095b65ae047f, test:rust/crates/urzua-cli/tests/init_greenfield_integration.rs#record_shaped_files_outside_the_selected_types_are_reported_not_errored@sha256:c2a9685601c3475b3bcc663e66fe30be3750aa55b888351a517f2599078af0c3
- The open questions listed in the What (CI and hook wiring, the display-number scheme, rule inference, one `.urzua/` per monorepo) are each decided or recorded. — evidence: spec:docs/specs/SPEC-5-urzua-init.md
- `SPEC-5` describes exactly what ships. — evidence: spec:docs/specs/SPEC-5-urzua-init.md

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
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-13 | Built: greenfield `init` with `--types`/`--dir`, the `adr`, `rfc`, `spec` and `prd` profiles written out into the config, starter templates, the interactive menu, and adopt's notice for a root-level record; the four open questions are decided or recorded in `SPEC-5`. Evidence added to each Done item. **Why:** Maintainer decision, 2026-11-13: the profile contents, `Stable-Id` required in every profile, `prd` provisional, templates written by default, and a file `init` cannot classify is a notice with no exit-code change. | **substantive** |
> | 2026-11-13 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (7 of 7 items resolved) computes to Specified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
