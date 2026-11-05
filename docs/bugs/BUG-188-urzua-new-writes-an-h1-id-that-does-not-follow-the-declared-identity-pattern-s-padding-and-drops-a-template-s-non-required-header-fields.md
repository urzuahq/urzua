---
Stable-Id: 01M6JW438QHFRMZPF22SZM9JAM
Status: Fixed
Found-in: 'a re-run of urzua over a real adopter corpus (2026-11-04)'
Realized-by: test:rust/crates/urzua-cli/tests/new_integration.rs#new_over_a_padded_identity_pattern_passes_check_with_no_edit_observed_failing@sha256:99269d48c00a5babfb8297e9f4d75a078cc369a34e23c093769bef3da39f4cf0, test:rust/crates/urzua-cli/tests/new_integration.rs#new_keeps_the_templates_declared_non_required_header_fields_observed_failing@sha256:17736e2e50757479f0df144720ce1500ea679129e336a89fa522060f416e179d
---
# 188 — Urzua new writes an H1 id that does not follow the declared identity pattern's padding and drops a template's non-required header fields

## What was wrong

Against a type whose identity pattern declares zero-padded numbers, `urzua new` writes a padded filename but an unpadded H1 id beside it, so the two disagree. It also drops header fields a type's template declares that are not required, so the new record starts without fields the template meant it to carry. `BUG-182` fitted the file name and H1 to the declared pattern and `MILE-151` made the generator follow the declared field set; neither covered the H1 padding nor the template's non-required fields.

## Why nothing caught it

The `BUG-182` tests ran `urzua new` then `check` and passed because the unpadded H1 satisfies the checker; none compared the H1 with the file name, and none used a template carrying optional header fields.

## References

Related: `BUG-182`, `MILE-151`. Done when `urzua new` writes an H1 id padded exactly as the file name is, keeps the template's non-required header fields, and this is exercised by a type with a padded pattern and an optional template field in urzua's own `.urzua/config.yaml`, with `urzua check` clean over the result.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this defect. | **substantive** |
> | 2026-11-04 | Fixed. The H1 id is padded to the width of the `number` the declared pattern reads from the file name written beside it (derived from config, not hardcoded); header fields the template names that the type declares but does not require are kept, blank. Exercised by `urzua new waiver` here (padded pattern, optional `Expires`). **Why:** `SPEC-12` 0.10. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 2 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
