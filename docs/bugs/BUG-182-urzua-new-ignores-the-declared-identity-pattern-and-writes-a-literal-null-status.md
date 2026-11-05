---
Stable-Id: 01M6JKNXV24H9XAAQW3NFJ7D5S
Status: Fixed
Found-in: 'a trial run over a real adopter corpus (2026-11-04)'
Realized-by: test:rust/crates/urzua-cli/tests/new_integration.rs#new_over_a_padded_identity_pattern_passes_check_with_no_edit_observed_failing@sha256:99269d48c00a5babfb8297e9f4d75a078cc369a34e23c093769bef3da39f4cf0, test:rust/crates/urzua-cli/tests/new_integration.rs#new_refuses_when_the_declared_pattern_cannot_name_the_file@sha256:5b8e55e5e88c9355910a0c14f05ddd2e3f72828ea9584f32226876f8179a539f, test:rust/crates/urzua-core/src/new_record.rs#a_declared_file_name_is_padded_to_the_width_the_pattern_requires@sha256:a9bb942fd06684f9daafa9b92cd2a7cc318f54ea16c6a451e613c7a629833ef5
---
# 182 — Urzua new ignores the declared identity pattern and writes a literal null status

## What was wrong

`urzua new adr "x"` against a type declaring an identity pattern with a four-digit number writes a file named with an unpadded number, an H1 with the number but no type prefix, a literal `Status: null`, and a `Stable-Id` the type's own closed field set rejects unless declared. The file then fails `filename.identity-pattern` and `field.quality`. `MILE-134` says the generated target must follow the declared pattern; it does not.

## Why nothing caught it

No test ran `urzua new` and then `check` over the result for a type with a non-default pattern, so the generator and the checker were never compared.

## References

BUG-2 of the adopter switchover report. Related gap: the filed `urzua new` honouring the declared pattern milestone; `MILE-134`.

Fixed with `MILE-151`: `urzua new` fits the file name and H1 to the declared pattern, starts the status field at the first declared status value, and omits `Stable-Id` (with a notice) when a closed `known_fields` does not list it. See `SPEC-12`.

## Regression test

rust/crates/urzua-cli/tests/new_integration.rs::new_over_a_padded_identity_pattern_passes_check_with_no_edit_observed_failing, rust/crates/urzua-cli/tests/new_integration.rs::new_leaves_out_a_stable_id_the_closed_field_set_does_not_declare_observed_failing, rust/crates/urzua-cli/tests/new_integration.rs::new_refuses_when_the_declared_pattern_cannot_name_the_file, rust/crates/urzua-core/src/new_record.rs::a_declared_file_name_is_padded_to_the_width_the_pattern_requires

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this defect. | **substantive** |
> | 2026-11-04 | Fixed with `MILE-151`. `Regression-test` filled. **Why:** the generator read a different grammar from the checker. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 3 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
