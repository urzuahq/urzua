---
Status: Done
Stable-Id: 01M1YWKN2HHC7DW4Y422DWJK1F
Phase: '1'
Track: schema-governance
Implements: RFC-10, ADR-38
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/header.rs#parse_with_shape@sha256:0684a0aa75d67805845cf344098f4076eb775d807219f0a18a524a5e8b40ecde, test:rust/crates/urzua-core/src/config.rs#a_removed_header_shape_is_a_clear_error_naming_the_form@sha256:3704ea8e26e1ead03470812212cd819ec5538a8d7452bbf58096c48dff815e80, code:rust/crates/urzua-core/src/config.rs#RecordTypeConfig::header_shape@sha256:a8cbdc7195689ce5ad34e2ec96b67345dabbb480c565097ec2fce90a44b9ba67
---
# 75 — Header sub-format consistency within a record type

## What

`HeaderShape::Blockquote` deliberately tolerates three sub-formats interchangeably (one-field-per-
line, bold-labelled, single-line pipe-delimited) so field extraction is robust to real-world
variance (RFC-10). That tolerance means `check` has no way to flag that SPEC-1 renders as multi-line
plain-label blockquote while SPEC-2 through SPEC-6 render as single-line pipe-delimited bold-label
blockquote — both parse correctly, so nothing fires, even though every other spec in the corpus
agrees on one visual family and SPEC-1 doesn't. Decide whether within-type sub-format drift deserves
its own (likely `Warning`-severity) rule, and if so, build it; if not, say explicitly why format
tolerance across sub-shapes is permanent rather than a maturity gap, the same way SPEC-1 already
does for structural-vs-content-scope.

## Why

Found live, reviewing SPEC-1 against SPEC-2 through SPEC-6: SPEC-1 has a `Version` field and
`check` reads it fine, but its layout has visibly drifted from every other spec in the corpus and
nothing says so. RFC-10's own three-shapes-recognized design was built to stop real corpus variance
from breaking field extraction — a reasonable goal — but as a side effect it also means the tool
can't tell a maintainer "these look inconsistent," which is a legitimate part of what "header
format consistency" (rules.rs's own doc comment for Rule 1) implies to a reader even though the
implementation only checks required-field presence per a declared shape, not sub-format uniformity.

## Done means

- A type's records agree on one header layout and `check` reports one that does not (the `header_layout` option and `header.layout-consistency` rule of `ADR-38`). — waived: not-applicable -- MILE-85 deleted the option, the rule and the sub-formats together with the shapes they policed, so no layout can differ; the criterion holds by construction (items 2 and 3)
- `yaml-frontmatter` is the only header shape. — evidence: code:rust/crates/urzua-core/src/header.rs#parse_with_shape@sha256:0684a0aa75d67805845cf344098f4076eb775d807219f0a18a524a5e8b40ecde, test:rust/crates/urzua-core/src/config.rs#a_removed_header_shape_is_a_clear_error_naming_the_form@sha256:3704ea8e26e1ead03470812212cd819ec5538a8d7452bbf58096c48dff815e80
- A type cannot mix layouts. — evidence: code:rust/crates/urzua-core/src/config.rs#RecordTypeConfig::header_shape@sha256:a8cbdc7195689ce5ad34e2ec96b67345dabbb480c565097ec2fce90a44b9ba67

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Fixed and shipped (ADR-38): a `header_layout` config field (`one-per-line` \| `pipe-delimited`), declared per type, checked by a new `header.layout-consistency` rule. **Why:** two cheaper alternatives (majority-vote inference, deriving from the type's checked-in template) were considered and rejected -- majority-vote ratifies existing drift instead of catching it, and template-derivation is silently inert for `spec`, which has no template yet (MILE-74) and is exactly the type that motivated this. Declaring it explicitly works regardless of template state. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Superseded by `MILE-85`; `Status` stays Done. **Why:** the closure audit of #234 swept the Done milestones for later removals. `header.layout-consistency`, the `header_layout` option and the sub-formats they policed were deleted by `MILE-85` together with the blockquote and bold-list shapes. The milestone delivered the rule when it shipped; the rule no longer exists. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
