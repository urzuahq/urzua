# 0016 — YAML frontmatter as the header shape for generated records

> Status: Accepted
> Date: 2026-09-05
> Author: (session author)
> Supersedes / Superseded-by: —
> Amends: RFC-0010

## Summary

Every header shape RFC-0010 declares so far — blockquote, bold-labelled blockquote, pipe-delimited,
bold-list — exists to correctly read a header some other convention already wrote by hand, before
this tool existed. None of them is a genuinely structured, machine-native format: each is text lines
a bespoke parser regexes apart, which is exactly why RFC-0010 needs a whole "closed structure" theory
(anchoring, closure, duplicate-key detection) to get right at all. That theory is only necessary
because the underlying format was never actually structured data.

Propose a fifth shape — YAML frontmatter, deserialized via `serde` into a real typed struct — as the
one Urzua itself writes for any record it generates going forward (`urzua new`, `urzua init`'s
templates). The existing four shapes are unaffected and keep exactly their current purpose: reading
a corpus that already uses a different convention.

## Motivation

A header is metadata: `Status`, `Date`, relationships (`Implements`, `Derives-from`), and — once
RFC-0005 lands — `realized_by` locators, waivers, and eventually claim-graph structures. None of that
is prose; all of it is exactly the shape a serialization format already solves for free. Continuing
to invent bespoke line-recognizers for each new shape (as ADR-0016 just did for bold-list) means
re-solving closure, typing, and nesting by hand every time a new convention is found, when a real
deserializer already has all three:

- **Closure is free.** A `serde` struct with `#[serde(deny_unknown_fields)]` already rejects an
  unrecognized key at parse time — no separate "is this the whole header, and only it" theory needed,
  because a YAML mapping's boundary (`---` to `---`) is unambiguous by construction, unlike a
  "maximal contiguous run of metadata-shaped lines" that has to be inferred line by line.
- **Typing is free.** A required field that's missing, or a field holding the wrong shape of value
  (a list where a string was expected), is a parse error from the deserializer, not a hand-written
  presence check per field.
- **Nesting is free.** RFC-0005's `realized_by` claim structures, and any future graph-shaped field,
  are just nested structs — no markdown-embedded syntax to invent for them, unlike trying to express
  an AND/OR composite inside a bold-list line.

This does **not** replace the four existing shapes or invalidate the work behind them. Adopting an
existing corpus (SPEC-0002 Phase C's whole falsifiability bar) means reading whatever convention that
corpus already uses — a real adopted corpus was never going to be in YAML frontmatter, since it
predates this tool. The shape-detection machinery stays exactly as necessary as it already was, for
exactly that job. What changes is only what Urzua itself writes when there's no existing convention to
match.

Body content — Context, Decision, Consequences, and any other prose section — is unaffected and stays
hand-authored markdown. Only the header block becomes structured; nothing about the "PR review reads
markdown" experience changes, since the same reviewer still reads the same rendered document, with a
metadata block at the top instead of a line-per-field block.

## Proposal

### 1. A fifth `HeaderShape` variant, additive

`HeaderShape::YamlFrontmatter` joins the existing four. Its region is the `---`-delimited block
starting at line 1 (a document not starting with frontmatter has no such region — same
`region: None` reportable-not-silent contract every other shape already honors). Parsing deserializes
the block into a typed struct rather than producing a flat `Vec<HeaderField>` line-by-line — a
deliberate difference from the other four shapes, since the whole point is to stop hand-rolling what a
deserializer already does correctly.

### 2. Declared per profile, defaulted for generation, not for reading

`header_shape` in `.urzua/config.toml` gains `"yaml-frontmatter"` as a legal value, consistent with
ADR-0016's existing declared-per-profile mechanism — no new declaration mechanism invented.
`urzua init`'s adopt mode continues to detect and preserve whatever shape a corpus already uses
(RFC-0010's existing job); `urzua init`'s from-scratch template generation and `urzua new` default to
`yaml-frontmatter` for any record type with no pre-existing convention to match.

### 3. Dependency choice: a pure-Rust, actively-maintained YAML implementation

Not `serde_yaml` (archived, deprecated). Verified before choosing: the crate must (a) work with the
already-pinned toolchain, and (b) introduce no C-toolchain dependency, since the release pipeline
cross-compiles to three targets (ADR-0013) and a C dependency is real cross-compilation risk. A
pure-Rust libyaml transpilation, maintained jointly by the original `serde_yaml` author and a
dedicated YAML-standard organization, round-trips correctly against a real struct and compiles clean
under the pinned toolchain with no C toolchain involved — the concrete bar any alternative would need
to clear.

## Consequences

- `urzua-core` gains a YAML-parsing dependency for the first time. Verify it stays outside
  `urzua-core`'s pure-dependency allowlist violation surface (ADR-0005/ADR-0006's mechanical purity
  test) — a YAML deserializer is not I/O-capable, so this should pass without changes to the
  allowlist, but the purity test is the thing that actually proves it, not this sentence.
- `Header`'s internal representation needs a variant for "parsed as a typed struct" alongside the
  existing "parsed as a flat field list" — the two other shapes' consumers (`header.get(key)`,
  duplicate-key detection) need a compatibility path so existing rules keep working against either
  representation without a rule-by-rule rewrite.
- Every template `urzua init`/`urzua new` generates changes from blockquote to YAML frontmatter.
  Existing adopted corpora are unaffected — their declared `header_shape` doesn't change unless a
  human edits config to opt in.
- This repo's own corpus (`urzuahq/urzua`'s `docs/`) was itself authored under the blockquote/bold-list
  shapes before this RFC. Migrating it is a separate, explicit decision — not implied by this RFC,
  which only changes the default for newly-generated records.

## References

- RFC-0010 — the header-shape/closure model this RFC amends by adding a fifth shape, not replacing
  the existing four.
- ADR-0016 — the declared-per-profile `header_shape` mechanism this RFC's new value slots into.
- RFC-0001 §1a, RFC-0005 — the relationship/claim-graph fields motivating a genuinely structured
  format rather than another bespoke line-recognizer.
- ADR-0001, ADR-0005/0006, ADR-0013 — the pure-core and cross-compilation constraints the dependency
  choice was verified against.
