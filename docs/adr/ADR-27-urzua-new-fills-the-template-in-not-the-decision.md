---
Stable-Id: 01M6R3NK4FPE25JR2HC916Z5EB
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/new_record.rs@sha256:21f5012c2b5ff34c2492d7ad9ca7c0a0a35fdc743df9fc76ae124a8210430e67, code:rust/crates/urzua-cli/src/commands/new.rs@sha256:e4ca70265a4531fd2025aa76cb0d510f73048a766510df430dd45616b787f581, test:rust/crates/urzua-core/src/new_record.rs@sha256:21f5012c2b5ff34c2492d7ad9ca7c0a0a35fdc743df9fc76ae124a8210430e67
Date: 2026-09-06
Accepted-on: 2026-09-06
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: ADR-3, ADR-21
---
# 27 — `urzua new` fills in the template, not the decision

## Context

SPEC-1 already describes `new`'s contract: "creates a record from the configured template with a
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

## Amendment (2026-11-04): the shapes it names are gone, author tiering is `gh` first, numbering skips every claimed ref

*Status of this amendment: accepted 2026-11-04 by the maintainer (drafted 2026-11-04 from the drift review the maintainer approved). The Decision text above is unchanged.*

Three statements in the Decision no longer match the code. The decision that `urzua new` fills in the
template and leaves `Status`, `Deciders` and `Embodiment` to the author is unchanged.

- **Header shapes.** The Decision refuses when a type with no template has a `blockquote` or
  `bold-list` shape. Those shapes no longer exist: `MILE-85` (#205) removed blockquote and bold-list
  parsing, so every record is YAML frontmatter and a type with no template always gets the
  synthesized header. The refusal branch is gone with them.
- **Author tiering.** The Decision resolves `Author` "explicit, then `gh api user`, then `git config
  user.name`". `urzua_io::resolve_identity` now tries `gh api user` first, then an explicit `--by`,
  then `git config user.name`, then an error; this is the order `ADR-31`'s amendment of 2026-09-10
  decided, and `ADR-20`'s amendment of 2026-11-04 records the same change for `fix`.
- **Numbering.** The Decision takes the highest numeric prefix in the type's directory plus one. `new`
  now also skips any number already claimed on a local or remote-tracking git ref and says which ref
  claimed it (`MILE-89`'s revision row of 2026-11-04, #135); a number claimed on a branch that was
  never fetched is still invisible. The declared `identity` pattern also decides the width of the
  number in the file name and the H1, and `Stable-Id` is always written (`MILE-151`, `BUG-188`).

## Amendment (2026-11-05): numbering spans every directory the type declares

*Status of this amendment: accepted 2026-11-05 by the maintainer (drafted 2026-11-05 at the maintainer's request). The Decision text above is unchanged.*

The 2026-11-04 amendment's Numbering bullet, and the Decision before it, say `new` numbers from "the
type's directory". Since `MILE-130` (#259), a record type may declare `extra_dirs` beside `dir`, and
every listed directory is one identity namespace. `new` therefore writes the record into `dir` but
numbers past the highest id found in **any** of the type's directories, and skips a number claimed in
any of them on another local or remote-tracking ref (`new.rs` `all_dirs()`). An `extra_dirs` entry
that does not exist holds no ids: `new` numbers without it and says so on stderr, and
`type.dir-matches-nothing` reports it.

The decision is unchanged: `urzua new` fills in the template and leaves `Status`, `Deciders` and
`Embodiment` to the author. A type that declares no `extra_dirs` numbers exactly as before.

## References

- SPEC-1 — `new`'s one-line contract this ADR fills in the mechanism for.
- ADR-3/21 — the stable-ID model `new` assigns from on creation, same as `migrate ids` backfills
  onto existing records.
- `rust/crates/urzua-core/src/fix.rs` — the pure-render/impure-caller split this module mirrors.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Realized-by` re-pointed at where the evidence now lives, each locator checked against the code it names. **Why:** the locators named files that have since been split or reorganised (`rules.rs` into `rules/<family>.rs`, `main.rs` into `commands/<command>.rs`), so the recorded `Embodiment` could no longer be re-verified; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit 4bc0874fc. | **structural** |
> | 2026-11-04 | Pinned `rust/crates/urzua-core/src/new_record.rs`, `rust/crates/urzua-cli/src/commands/new.rs` by content hash after re-verifying it against the Decision (attested by beauwilliams). **Why:** Re-verified: urzua new still fills the template and leaves the decision to the author; changes since are the declared identity pattern, padded H1, an always-written Stable-Id and the removal of the blockquote shapes. The Decision text on those shapes and author tiering is stale and is left for an amendment. | **structural** |
> | 2026-11-04 | Amendment added: the blockquote and bold-list refusal no longer applies (`MILE-85`), `Author` resolves `gh api user` first (`ADR-31`), and numbering skips numbers claimed on other refs and follows the declared identity pattern (`MILE-89`, `MILE-151`, `BUG-188`). **Why:** drift review of 2026-11-04, approved by beauwilliams; proposed, pending maintainer acceptance. The Decision is unchanged. | **substantive** |
> | 2026-11-04 | Amendment of 2026-11-04 accepted. **Why:** Maintainer decision, 2026-11-04; the Decision text is unchanged. | **substantive** |
> | 2026-11-05 | Amendment added: numbering spans every directory the type declares (`extra_dirs`, `MILE-130`); proposed, pending maintainer acceptance. **Why:** the drift check found `new.rs` numbering across `all_dirs()` while the text said "the type's directory" (#306). | **substantive** |
> | 2026-11-05 | Amendment of 2026-11-05 accepted. **Why:** Maintainer decision, 2026-11-05. | **substantive** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/new_record.rs` 15511d30876e -> 850e21078331, `rust/crates/urzua-cli/src/commands/new.rs` 1e4723c81adc -> c1190b45b353. Reason: new.rs now numbers across every directory the type declares (MILE-130, extra_dirs), which the accepted 2026-11-05 amendment records; new_record.rs only gained template_header_values for BUG-22 placeholder detection, additive and not on the numbering or fill path. The decision and its amendments hold **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/new_record.rs` 850e21078331 -> 21f5012c2b5f, `rust/crates/urzua-cli/src/commands/new.rs` c1190b45b353 -> 5b61c497ff7e. Reason: Read the diff of each pinned span since the old pin and the claim still holds. urzua-core/src/new_record.rs: next_display_number and the number assignment now return a Result and refuse an exhausted number instead of overflowing, new records may fill a declared title field and initial version, and a required Stable-Id is no longer overwritten with null, template filling is otherwise as before; commands/new.rs: gained the opt-in open-pull-request check (ADR-93): it fetches the pull request head refs, takes the numbering lock and reports a pull-refs block, template filling is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/new.rs` 5b61c497ff7e -> e4ca70265a45. Reason: Read the diff of each re-pinned file against main. cli/src/commands/new.rs: Notice struct literals gained detail: None; no behaviour change. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
