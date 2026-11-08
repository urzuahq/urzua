---
Stable-Id: 01M6D5VJX3CC14QH9FKNHYZZBZ
Status: Fixed
Found-in: "A /code-review pass, round 30"
Realized-by: test:rust/crates/urzua-core/src/config.rs#an_unsafe_prefix_is_refused_at_load@sha256:11e979ef89bbfd33e400c33bcb40db97e5ca39fbcada323c6857727e2ce1c82a, test:rust/crates/urzua-core/src/config.rs#two_types_sharing_a_prefix_are_refused_at_load@sha256:06513e713838feb6655d9493f8e0b0908621ad46f393707de06536a147f03b29, test:rust/crates/urzua-core/src/config.rs#an_empty_or_root_dir_is_refused_at_load@sha256:0700ce3b71e6a72be766c0428e196fdcc64aa4f77e61e267fea09dd1f4fa9cce, test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_reports_an_invalid_record_type_prefix_or_dir@sha256:0cf80deb45aac8cd65dde8413b0f55e9df06f475715d63a72ba8add516c29bf8
---
# 173 — record-type prefix and dir are accepted unvalidated at config load so new writes unrecognizable or colliding records and an empty dir examines nothing

## What was wrong

Three record-type config values reached filesystem-facing code with no validity check at load:

- `prefix` accepted `""`, `../..`, `a b`, `ADR-`, `A/B`. `urzua new` then wrote `docs/adr/-6-x.md`,
  `docs/..-6-x.md` (outside the type's dir), `docs/adr/a b-6-x.md`, or failed with an OS error; `check`
  could not recognize any of them as the type's record, so the record was created but invisible.
- Two types could declare the same `prefix`; `urzua new` numbers per type, so it minted `PFX-2` twice and
  `identity.collision` then blocked the corpus the tool itself had built.
- `dir: ""` or `"."` resolved to the repository root: `check` examined 0 files and exited 2 not-run with
  no stated cause, every rule reported `ran`, and `doctor` said the dir "exists".

## Why nothing caught it

One shape: values the config declares were trusted by the code that consumes them. Validation of rule
levels and options existed (`UnknownRule`, `OptionRequired`) but nothing covered record-type values.

## Fix

`config::parse` now validates every record type, once, so every command and `doctor` (which loads
through `parse`) agree. `ConfigError::InvalidPrefix` reuses the filename grammar's own predicates
(`is_prefix_segment`, `is_digit_segment`): hyphen-separated, non-empty, upper-case/digit segments, none
all-digit. `ConfigError::DuplicatePrefix` compares effective prefixes (explicit, or the upper-cased type
name). `ConfigError::InvalidDir` rejects an empty, blank, or `.`/`./` dir. `BUG-168`'s `./`-in-the-middle
and shared-`dir` cases are not touched. Consequence: a shared prefix is no longer a way to plant a
cross-type identity collision, so the integration fixtures that did so now plant a same-type duplicate.

Doctor reports these as `config-parses` errors (exit 1).

## References

`BUG-168` (open) owns the `./` and shared-`dir` cases. `BUG-114` (prefix grammar).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed and fixed. **Why:** found by a round 30 review and reproduced against a disposable fixture. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 4 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#doctor_reports_an_invalid_record_type_prefix_or_dir` (lines 365-392) 848cc9708286 -> 0cf80deb45aa. Reason: Read the diff of each pinned span since the old pin and the claim still holds. check_integration.rs#doctor_reports_an_invalid_record_type_prefix_or_dir: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
