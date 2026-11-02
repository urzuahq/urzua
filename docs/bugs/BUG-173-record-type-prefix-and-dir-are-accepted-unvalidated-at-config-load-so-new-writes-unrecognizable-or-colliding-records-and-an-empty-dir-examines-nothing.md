---
Stable-Id: 01M6D5VJX3CC14QH9FKNHYZZBZ
Status: Fixed
Found-in: "A /code-review pass, round 30"
Regression-test: "config::tests::an_unsafe_prefix_is_refused_at_load, two_types_sharing_a_prefix_are_refused_at_load, an_empty_or_root_dir_is_refused_at_load; check_integration doctor_reports_an_invalid_record_type_prefix_or_dir"
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
