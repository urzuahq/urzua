---
Stable-Id: 01M2P88HFRZDB9R8K03GBVBAGE
Status: Accepted
Date: 2026-09-16
Author: beauwilliams
---
# 31 — A record type must be able to declare that its metadata is not in a header

## Summary

`header_shape` offers three values — `yaml-frontmatter`, `blockquote`, `bold-list` — and no way to say
"this type has no header block." A corpus whose metadata lives in prose and sections cannot be
expressed, and `check` reports nine errors against nine perfectly valid records.

## Motivation

From `MILE-51`, run against `npryce/adr-tools`. A Nygard record looks like this:

```markdown
# 1. Record architecture decisions

Date: 2016-02-12

## Status

Accepted
```

`Date:` is a bare line. Status is a section. There is no header region in any declared shape, so every
record reports:

```
no header-shaped region found -- required fields [] cannot be checked
```

`required fields []` — nothing was required, and it failed anyway. Nine errors, `blocking: true`, on
a corpus that is entirely valid in its own convention.

**Declaring `required_fields = []` does not express this**, which is the obvious workaround and the
reason this needs a decision rather than a patch. `header.required-fields` fails on the missing
*region*, not on unsatisfied fields. And a blanket "skip when nothing is required" guard cannot be the
answer: `check_integration.rs:380-416` exists specifically to assert the opposite, because `init`
writes `required_fields = []` and `header_shape = "yaml-frontmatter"` for every adopted corpus
(`init.rs:92`, `:97`, deliberate per `ADR-33`) — so that finding is how an adopter learns their
declared shape does not match their corpus. Silencing it silences the adoption signal for its only
audience.

So the tool must distinguish two situations it currently cannot:

- **"my records have no header"** — legitimate, should be quiet
- **"my declared shape does not match my records"** — a real defect, must stay loud

Only a declaration can tell them apart.

## Proposal

Sketch, not settled. A fourth value:

```toml
[record_types.adr]
header_shape = "none"
```

Under it, `header.required-fields` skips the type entirely rather than reporting a missing region,
matching how `header.layout-consistency` (`rules.rs:97-108`) and `header.field-set-consistency`
(`rules.rs:156-168`) already skip a type that declares nothing — both with doc comments saying
`examined` stays 0 for it.

## Open questions

- **`required_fields` must then be empty.** Is declaring both a config-validation error, like
  `config.pointer-narrative-overlap`, or is a non-empty list under `none` simply unsatisfiable and
  therefore already an error by another name?
- **`header.deprecated-shape`** (`rules.rs:263`) reports types still on `blockquote`/`bold-list`. Does
  `none` read as deprecated, as current, or as outside that axis entirely?
- **What does `init` propose?** `ADR-33` has it always propose `yaml-frontmatter` — "where to grow",
  not a preservation of what exists. Does an adopted headerless corpus get `none` (accurate, and the
  records stay quiet) or `yaml-frontmatter` (aspirational, and every record errors until migrated)?
  This is the crux, and it is really a question about what adopt mode is *for*.
- **Does `urzua new` refuse** for a `none` type, since it has no header to render? Or does the type
  become read-only, checkable but not writable?
- **Is this the right axis at all?** The deeper reading is that a Nygard record's metadata *is*
  declared — in sections (`MILE-4`) and in a bare `Date:` line. `none` says "there is nothing here,"
  which is not quite true; "it is somewhere else" is closer. If section-declared fields land first,
  this may be the wrong shape.

## References

- MILE-51 -- the validation run this comes from; four sibling gaps.
- ADR-33 -- `init` always proposes `yaml-frontmatter`; the open question above is a direct consequence.
- MILE-4 -- section structure; where a Nygard record's metadata actually lives.
- RFC-32 -- the sibling "deliberately none" gap, same run, same shape of problem.
- ADR-16, RFC-10 -- header shapes are declared per profile, never sniffed; this adds a value, not a
  sniffing mechanism.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | `Draft` -> `Accepted`, decided by `ADR-50`. **Why:** decided before building, rather than patching around the gap -- the open questions this proposal listed are answered in the decision, including one it did not anticipate. | **substantive** |
> | 2026-09-16 | Initial proposal, `Status: Draft`. **Why:** measured, not predicted -- nine errors against nine valid records, and the obvious workaround (`required_fields = []`) is already asserted *not* to work by an existing integration test whose comment explains why. | **structural** |
