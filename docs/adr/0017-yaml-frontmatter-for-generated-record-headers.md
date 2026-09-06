# 0017 — YAML frontmatter for generated record headers

> Status: Accepted
> Embodiment: Not started
> Date: 2026-09-05
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-0016 (Accepted)

## Context

RFC-0010's four declared header shapes (blockquote, bold-blockquote, pipe-delimited, bold-list) all
exist to read a convention some other author already wrote by hand, and each is a bespoke
line-recognizer rather than a genuinely structured format — which is why RFC-0010 needs a whole
closure theory (anchoring, duplicate-key detection, region boundaries) to get right. RFC-0016 proposed
a fifth shape, YAML frontmatter deserialized via a real typed struct, as what Urzua itself writes for
any record it generates, while leaving the existing four exactly as necessary for adopting a corpus
that already uses one of them.

## Decision

In the context of a header format that is metadata (relationships, status, eventually claim-graph
structures) rather than prose, facing the cost of hand-rolling closure/typing/nesting anew for every
shape RFC-0010 adds, we decided that **`urzua new` and `urzua init`'s from-scratch templates generate
YAML frontmatter**, deserialized by `serde` into a typed struct rather than a flat field list. Adopting
an existing corpus is unaffected: `init`'s adopt mode continues to detect and preserve whatever shape
that corpus already uses.

The YAML crate is `yaml_serde` — verified before choosing, not assumed: it round-trips correctly
against a real struct, compiles under the exact pinned toolchain (1.87.0) with no version bump, and
introduces no C-toolchain dependency (it is a pure-Rust transpilation of libyaml, not a binding to it),
which matters directly because the release pipeline (ADR-0013) cross-compiles to three targets.
`serde_yaml` (archived) and `serde-saphyr` (requires rustc 1.89, forcing an unplanned toolchain bump)
were both ruled out on verified, concrete grounds, not preference.

## Reversibility

Additive: a fifth `HeaderShape` variant alongside the existing four, not a replacement. No existing
adopted corpus's declared shape changes unless a human edits its config. This repo's own corpus
predates this decision and is not migrated by it — that stays a separate, explicit choice.

## Consequences

- `urzua-core` takes on its first YAML-parsing dependency. The mechanical purity test (ADR-0005/0006)
  must still pass — a deserializer is not I/O-capable, but this is asserted by the test, not by this
  sentence, and must be re-run once the dependency lands.
- `Header`'s representation needs to serve both a flat-field-list consumer (the existing four shapes)
  and a typed-struct consumer (frontmatter) without forcing every existing rule to be rewritten
  per-shape.
- Every newly-generated record's header looks structurally different from this project's own existing
  33 records — a visible, deliberate discontinuity between "how this repo happens to be written today"
  and "what a new record looks like going forward," not a retroactive rewrite.

## References

- RFC-0016 — the proposal this decides.
- RFC-0010 — the four existing shapes, unaffected, still necessary for adoption.
- ADR-0016 — the declared-per-profile `header_shape` mechanism this reuses.
- ADR-0005, ADR-0006, ADR-0013 — the purity and cross-compilation constraints the dependency choice
  was checked against.
