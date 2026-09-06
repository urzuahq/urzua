# 0016 — A bold-list header shape, declared per profile

> Status: Accepted
> Embodiment: Verified
> Realized-by: code:rust/crates/urzua-core/src/header.rs, test:rust/crates/urzua-core/src/header.rs
> Date: 2026-09-05
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-0010 (Accepted)

## Context

RFC-0010 left open where a second header shape fits: a second declared shape per profile, or a
normalization step before parsing. Running `urzua check` against real corpora this project doesn't
control (issue `urzuahq/urzua#1`) found real records whose header fields sit in a plain markdown list
with no blockquote wrapper — `- **Key:** Value` directly under the H1, not `> **Key:** Value` — which
the existing recognizer cannot see at all, since it anchors on a leading `>` unconditionally.

A second shape, heading-delimited fields (each field is its own `##` section, its value the section
body), was found in the same round of testing but does not fit this decision: a field's value there
spans a whole body region rather than one line, which changes what `Header.region` means rather than
adding a new line-recognizer to the existing model.

## Decision

In the context of RFC-0010's open question on where additional header shapes fit, facing real
evidence for a bold-list shape that fits the current line-based model cleanly, we decided to add
`HeaderShape::BoldList` as a second declared shape alongside the existing blockquote shape —
declared per record type via `header_shape` in `.urzua/config.toml` (`"blockquote"` | `"bold-list"`),
defaulting to `"blockquote"` so no existing config needs to change. The shape is never sniffed from
content: a misconfigured shape must fail loud (`no header-shaped region found`) rather than silently
matching the wrong lines, consistent with RFC-0010 §2's rejection of positional/sniffing heuristics
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
  fail-loud precedent (ADR-0012) rather than silently falling back to a default.
- `header::parse_with_shape` dispatches to a shape-specific line recognizer (`blockquote_body` or
  `bold_list_body`) but reuses the same `parse_metadata_line`/`parse_key_value` machinery for both —
  the shapes differ only in how a field-bearing line is recognized, not in how its key/value are
  parsed.
- The heading-delimited shape remains an open item, tracked against `urzuahq/urzua#1`, needing its
  own `Header.region` redesign before it can be declared alongside these two.
- SPEC-0003's configuration surface documents `header_shape` as part of the per-record-type schema.

## References

- RFC-0010 — the open question this ADR resolves for the bold-list case only.
- `urzuahq/urzua#1` — the real foreign-corpus evidence for both shapes.
- `rust/crates/urzua-core/src/header.rs` — the implementation.
