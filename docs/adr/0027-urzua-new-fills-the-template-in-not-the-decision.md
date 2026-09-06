# 0027 — `urzua new` fills in the template, not the decision

> Status: Accepted
> Embodiment: Verified
> Realized-by: code:rust/crates/urzua-core/src/new_record.rs, code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-core/src/new_record.rs
> Date: 2026-09-06
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: ADR-0003 (Accepted), ADR-0021 (Accepted)

## Context

SPEC-0001 already describes `new`'s contract: "creates a record from the configured template with a
stable ID assigned. Never asks the author to pick a number. Prints the path." Two questions were
still open in practice: what happens when a record type has no `.urzua/templates/<type>.md` (this
repo's own three types all have one, but a type declaring `header_shape = "yaml-frontmatter"` has no
reason to keep a hand-maintained template around), and which parts of a template get filled in
automatically versus left as the author's own placeholder text to edit.

## Decision

In the context of a record type either carrying a checked-in template or declaring
`yaml-frontmatter`, facing the same fork `header.rs`'s three shapes already draw, we decided:

- **If `.urzua/templates/<type>.md` exists, fill it in.** The H1's number and title, `Date`,
  `Author` (resolved the same way `fix --apply` resolves an identity: explicit, then `gh api user`,
  then `git config user.name`), and a freshly generated `Stable-Id` line inserted right after the
  header's first line. Everything else — `Status: Proposed | Accepted | ...`, `Deciders`, an
  `Embodiment` starting value — stays exactly the enumerated placeholder text the template already
  has, because those are decisions a human makes, not values `new` can compute.
- **If no template exists and the type declares `yaml-frontmatter`, synthesize one** from
  `required_fields`: every configured field emitted as a blank frontmatter key, so the config already
  drives what an author has to fill in without a template file needing to exist at all.
- **If neither holds, refuse** rather than guess a shape. A record type with no template and a
  `blockquote`/`bold-list` shape has nothing for `new` to safely generate a header from.
- **The display number is computed by scanning the type's directory for the highest existing
  numeric prefix and taking the next integer** — never reused even if a number was deleted, since a
  cross-reference elsewhere may still assume the old numbering held.

## Reversibility

Additive: a new pure module (`new_record.rs`, following the same pure-render/impure-caller split as
`fix.rs`) and one new CLI command. Nothing existing changes shape.

## Consequences

- `new` never writes `Status`, `Deciders`, or `Embodiment`'s starting value — a generated record is
  never silently `Accepted` or `Verified` by construction.
- A record type declaring neither a template nor `yaml-frontmatter` cannot use `new` yet. That is a
  real, named gap (a fourth option — synthesize a `blockquote` header from `required_fields` alone,
  the way the YAML path does — is a plausible follow-up, not ruled out here, just not needed by any
  type this repo currently declares).
- The identity-resolution reuse means `new` and `fix --apply` share one failure mode and one
  precedent: no `--by`, no `gh`, no `git config user.name` — refuse, don't guess an author.

## References

- SPEC-0001 — `new`'s one-line contract this ADR fills in the mechanism for.
- ADR-0003/0021 — the stable-ID model `new` assigns from on creation, same as `migrate ids` backfills
  onto existing records.
- `rust/crates/urzua-core/src/fix.rs` — the pure-render/impure-caller split this module mirrors.
