---
Stable-Id: 01M6R3NK46ZQ08SSX6286KEZWZ
Status: Retired
Embodiment: Drift detected
Realized-by: code:rust/crates/urzua-core/src/header.rs, test:rust/crates/urzua-core/src/header.rs
Date: 2026-09-05
Accepted-on: 2026-09-05
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: RFC-10
---
# 16 — A bold-list header shape, declared per profile

## Context

RFC-10 left open where a second header shape fits: a second declared shape per profile, or a
normalization step before parsing. Running `urzua check` against real corpora this project doesn't
control (issue `urzuahq/urzua#1`) found real records whose header fields sit in a plain markdown list
with no blockquote wrapper — `- **Key:** Value` directly under the H1, not `> **Key:** Value` — which
the existing recognizer cannot see at all, since it anchors on a leading `>` unconditionally.

A second shape, heading-delimited fields (each field is its own `##` section, its value the section
body), was found in the same round of testing but does not fit this decision: a field's value there
spans a whole body region rather than one line, which changes what `Header.region` means rather than
adding a new line-recognizer to the existing model.

## Decision

In the context of RFC-10's open question on where additional header shapes fit, facing real
evidence for a bold-list shape that fits the current line-based model cleanly, we decided to add
`HeaderShape::BoldList` as a second declared shape alongside the existing blockquote shape —
declared per record type via `header_shape` in `.urzua/config.toml` (`"blockquote"` | `"bold-list"`),
defaulting to `"blockquote"` so no existing config needs to change. The shape is never sniffed from
content: a misconfigured shape must fail loud (`no header-shaped region found`) rather than silently
matching the wrong lines, consistent with RFC-10 §2's rejection of positional/sniffing heuristics
generally.

The heading-delimited shape is explicitly deferred rather than bundled into this decision: it needs
its own `Header.region` model (a per-field span, not one contiguous line range), which is a
structural change deserving its own design pass rather than a rushed extension of this one.

## Reversibility

Additive: `header_shape` is a new, optional config field with a backward-compatible default, and
`HeaderShape::BoldList` is a new enum variant alongside the existing behavior, not a replacement for
it. `Record::parse` keeps its existing signature via a new `parse_with_shape` constructor, so no
existing caller needs to change.

## Consequences

- `.urzua/config.toml`'s `record_types.<name>.header_shape` accepts `"blockquote"` (default) or
  `"bold-list"`; an unrecognized value is a parse-time error, matching `schema_version`'s existing
  fail-loud precedent (ADR-12) rather than silently falling back to a default.
- `header::parse_with_shape` dispatches to a shape-specific line recognizer (`blockquote_body` or
  `bold_list_body`) but reuses the same `parse_metadata_line`/`parse_key_value` machinery for both —
  the shapes differ only in how a field-bearing line is recognized, not in how its key/value are
  parsed.
- The heading-delimited shape remains an open item, tracked against `urzuahq/urzua#1`, needing its
  own `Header.region` redesign before it can be declared alongside these two.
- SPEC-3's configuration surface documents `header_shape` as part of the per-record-type schema.
- **Narrowed by ADR-33**: bold-list is deprecated in favor of `yaml-frontmatter` — its structural
  ceiling is the same as blockquote's (a flat one-line-per-field model, no path to RFC-5's nested
  claim graph). Parsing support stays; nothing in this ADR's actual decision is reversed, only its
  standing as a recommended shape going forward.

## Amendment (2026-11-08): the bold-list shape was removed from the engine

*Status of this amendment: accepted 2026-11-08 by the maintainer. The Decision text above is unchanged; see the revision log.*

The Decision above adds `HeaderShape::BoldList` as a declared per-profile shape, and the Consequences
keep its parsing support ("Parsing support stays"). That is no longer true. A breaking change on `main`, released in the
next minor version (0.4.0, not yet tagged on 2026-11-08), removed the `bold-list` shape, together with the `blockquote` shape, so the engine has no reader for
`- **Key:** Value` headers: `header_shape` accepts `yaml-frontmatter` (the default when omitted) or
`none`, a config declaring `header_shape: bold-list` fails to load with an error naming the removed
shape, and a record still written in that form is reported by `header.required-fields` as unreadable
with the form named. The change landed in `MILE-85` (`#205`) and is described to users by the
changeset `.changeset/remove-blockquote-and-bold-list-headers.md` (a major bump, closing `BUG-184`).

The reason is the one the narrowing by `ADR-33` already gave: the shape has the same structural ceiling
as blockquote, a flat one-line-per-field model with no path to the nested claim graph of `RFC-5`, and
once every record type in the corpus was `yaml-frontmatter` it had no user left to justify keeping a
second and third recognizer. No decision replaces it: a header is YAML frontmatter, so there is no
shape left to declare, and `ADR-33` is the closest record. The heading-delimited shape this ADR
deferred remains unbuilt. `Status` is left unchanged here for the maintainer to retire separately, as
was done for `ADR-38`.

## References

- RFC-10 — the open question this ADR resolves for the bold-list case only.
- `urzuahq/urzua#1` — the real foreign-corpus evidence for both shapes.
- `rust/crates/urzua-core/src/header.rs` — the implementation.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Embodiment` recomputed by `urzua fix`: `Drift detected`. **Why:** every file named in `Realized-by` has changed since this line was last verified, and this pass did not re-verify it, so the record states the drift instead of a stale `Verified`; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-core/src/header.rs, test:rust/crates/urzua-core/src/header.rs` -- stated `Verified` disagreed with the computed value. | **structural** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit 895cf4ca3. | **structural** |
> | 2026-11-08 | Amendment added saying the `bold-list` header shape was removed from the engine by a breaking change on `main`, to ship in 0.4.0 (`MILE-85`, `#205`; changeset `remove-blockquote-and-bold-list-headers`), and accepted by the maintainer on 2026-11-08. **Why:** maintainer decision, 2026-11-08: the Decision's mechanism and its `Parsing support stays` consequence no longer hold, and the record should say so before it is retired as `ADR-38` was; the Decision is unchanged and `Status` is not changed by this row. | **substantive** |
> | 2026-11-08 | `Status` changed from Accepted to Retired. **Why:** maintainer decision, 2026-11-08: the mechanism this decision governs was removed with no successor record, as with `ADR-38`. | **substantive** |
