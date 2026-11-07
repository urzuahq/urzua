---
Status: Done
Stable-Id: 01M1Y5JCCP2D9ES98WDB9V5PPF
Phase: '0'
Track: header-format
Implements: ADR-27, BUG-3
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/new.rs, code:rust/crates/urzua-core/src/new_record.rs, test:rust/crates/urzua-cli/tests/new_integration.rs#new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present@sha256:abbf083803b8a4fe62e6e7b933ffdec9e9964180b452f0a00334eaca302cfbd0
---
# 1 — Fix urzua new's template-priority bug

## What

urzua new picks blockquote vs. YAML frontmatter by whether a template file exists, ignoring the configured header_shape entirely.

## Why

Flipping any type's header_shape to yaml-frontmatter without this fix means urzua new keeps emitting blockquote content that check can't parse under the declared shape -- silently broken output.

## Done means

- `urzua new` for a type configured `yaml-frontmatter` emits a YAML-frontmatter record whether or not a blockquote-shaped template file exists. — evidence: code:rust/crates/urzua-cli/src/commands/new.rs, code:rust/crates/urzua-core/src/new_record.rs, test:rust/crates/urzua-cli/tests/new_integration.rs#new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present@sha256:abbf083803b8a4fe62e6e7b933ffdec9e9964180b452f0a00334eaca302cfbd0
- A test plants a blockquote-shaped template and fails if the output is blockquote (`new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present`). — evidence: test:rust/crates/urzua-cli/tests/new_integration.rs#new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present@sha256:abbf083803b8a4fe62e6e7b933ffdec9e9964180b452f0a00334eaca302cfbd0
- The configured `header_shape` is the only thing that picks the output shape. — evidence: code:rust/crates/urzua-cli/src/commands/new.rs

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Fixed and shipped: header_shape wins unconditionally over a template's own shape (BUG-3). | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
