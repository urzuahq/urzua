---
Stable-Id: 01M6R3NK47HQY1YTRDG8MG679M
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/header.rs@sha256:138444217009c90bb532bed4f31605269e6a53ccde85651e142fc1da5ea1e654, test:rust/crates/urzua-core/src/header.rs@sha256:138444217009c90bb532bed4f31605269e6a53ccde85651e142fc1da5ea1e654
Date: 2026-09-05
Accepted-on: 2026-09-05
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: RFC-16
---
# 17 — YAML frontmatter for generated record headers

## Context

RFC-10's four declared header shapes (blockquote, bold-blockquote, pipe-delimited, bold-list) all
exist to read a convention some other author already wrote by hand, and each is a bespoke
line-recognizer rather than a genuinely structured format — which is why RFC-10 needs a whole
closure theory (anchoring, duplicate-key detection, region boundaries) to get right. RFC-16 proposed
a fifth shape, YAML frontmatter deserialized via a real typed struct, as what Urzua itself writes for
any record it generates, while leaving the existing four exactly as necessary for adopting a corpus
that already uses one of them.

## Decision

In the context of a header format that is metadata (relationships, status, eventually claim-graph
structures) rather than prose, facing the cost of hand-rolling closure/typing/nesting anew for every
shape RFC-10 adds, we decided that **`urzua new` and `urzua init`'s from-scratch templates generate
YAML frontmatter**, deserialized by `serde` into a typed struct rather than a flat field list. Adopting
an existing corpus is unaffected: `init`'s adopt mode continues to detect and preserve whatever shape
that corpus already uses.

The YAML crate is `yaml_serde` — verified before choosing, not assumed: it round-trips correctly
against a real struct, compiles under the exact pinned toolchain (1.87.0) with no version bump, and
introduces no C-toolchain dependency (it is a pure-Rust transpilation of libyaml, not a binding to it),
which matters directly because the release pipeline (ADR-13) cross-compiles to three targets.
`serde_yaml` (archived) and `serde-saphyr` (requires rustc 1.89, forcing an unplanned toolchain bump)
were both ruled out on verified, concrete grounds, not preference.

## Reversibility

Additive: a fifth `HeaderShape` variant alongside the existing four, not a replacement. No existing
adopted corpus's declared shape changes unless a human edits its config. This repo's own corpus
predates this decision and is not migrated by it — that stays a separate, explicit choice.

## Consequences

- `urzua-core` takes on its first YAML-parsing dependency. The mechanical purity test (ADR-5/6)
  must still pass — a deserializer is not I/O-capable, but this is asserted by the test, not by this
  sentence, and must be re-run once the dependency lands.
- `Header`'s representation needs to serve both a flat-field-list consumer (the existing four shapes)
  and a typed-struct consumer (frontmatter) without forcing every existing rule to be rewritten
  per-shape.
- Every newly-generated record's header looks structurally different from this project's own existing
  33 records — a visible, deliberate discontinuity between "how this repo happens to be written today"
  and "what a new record looks like going forward," not a retroactive rewrite.

## References

- RFC-16 — the proposal this decides.
- RFC-10 — the four existing shapes, unaffected, still necessary for adoption.
- ADR-16 — the declared-per-profile `header_shape` mechanism this reuses.
- ADR-5, ADR-6, ADR-13 — the purity and cross-compilation constraints the dependency choice
  was checked against.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Embodiment` recomputed by `urzua fix`: `Drift detected`. **Why:** every file named in `Realized-by` has changed since this line was last verified, and this pass did not re-verify it, so the record states the drift instead of a stale `Verified`; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-core/src/header.rs, test:rust/crates/urzua-core/src/header.rs` -- stated `Verified` disagreed with the computed value. | **structural** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit 1449f310a. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/header.rs@sha256:6bce44f1b7cf, rust/crates/urzua-core/src/header.rs@sha256:6bce44f1b7cf Reason: Pinned the files as they are now after checking each against the record it evidences: header.rs still parses the declared YAML frontmatter shape (ADR-17), fix.rs still has detect_repairs (ADR-19), urzua-id still builds ULID stable ids (ADR-21), main.rs still prints JSON with no --format flag on check (ADR-23), urzua-io still resolves the attesting identity in the ADR-31 order, scripts/generate-dashboard.py still generates the static dashboard (ADR-37), report.rs still defines Report and Notice and main.rs still emits them (ADR-46), publish-release.yml still verifies the tree it ships through checks.yml (ADR-47), knope.toml still releases from changesets (ADR-48), and ci.yml still gates changesets on .github/release-paths (ADR-49). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-core/src/header.rs@sha256:6bce44f1b7cf05655c37ba0dcd6e2d5d254f965249950a6d50b3ec1314a9fced, test:rust/crates/urzua-core/src/header.rs@sha256:6bce44f1b7cf05655c37ba0dcd6e2d5d254f965249950a6d50b3ec1314a9fced` -- stated `Drift detected` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/header.rs` 6bce44f1b7cf -> 138444217009. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: header.rs: split parse_yaml_frontmatter into header_without_region and frontmatter_fields; the opening-line, closing-line and YAML-error checks stay in parse_yaml_frontmatter in the same order with the same parse_error text. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline and reverify JSON compared before and after, with and without --as-of). **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
