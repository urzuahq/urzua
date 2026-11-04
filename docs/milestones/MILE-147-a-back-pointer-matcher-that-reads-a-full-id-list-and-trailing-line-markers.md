---
Stable-Id: 01M6JKNN0TRS89VT42DADJY1T5
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/back_pointer.rs, test:rust/crates/urzua-cli/tests/back_pointer_integration.rs
---
# 147 — A back-pointer matcher that reads a full id list and trailing-line markers

## What

Make the back-pointer matcher continue past a non-id token (`Implements: ADR-1 (note), ADR-2` must yield both ids) and accept a marker that follows code on the same line, not only whole-line comments. Apply the same reading to `embodiment.back-pointer-decision-accepted`.

## Why

On a real adopter corpus the matcher stops at the first non-id token and ignores same-line markers: dozens of file-and-record pairs across dozens of files are lost, and several records lose all of their code evidence. A reader that silently drops a citation reports drift that is not there, and the same narrowness makes the decision-accepted check weaker than it looks. Evidence the author wrote must be read as written. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

An id list with a parenthetical note between ids yields every id; a marker after code on the same line is read; `embodiment.back-pointer-decision-accepted` judges the same widened set; tests cover each shape and a negative case that is not a marker; and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Matcher widened (`SPEC-2` `0.29`): a note between ids no longer ends the list, a trailing comment after code is read, `embodiment.back-pointer-decision-accepted` judges the same set; exercised by this repository's own back-pointers. **Why:** the Done-when asks for exactly this. Status unchanged; the maintainer closes. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #193: the back-pointer matcher reads a full id list and a trailing comment after code. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
