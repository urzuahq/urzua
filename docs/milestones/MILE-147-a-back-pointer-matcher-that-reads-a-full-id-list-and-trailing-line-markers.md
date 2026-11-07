---
Stable-Id: 01M6JKNN0TRS89VT42DADJY1T5
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/back_pointer.rs#scan_marker@sha256:00c7620d0730f97c044bc524253a99a171f956587671595a2b19b7358e92abee, test:rust/crates/urzua-core/src/rules/back_pointer.rs#an_id_list_continues_past_a_parenthetical_note_observed_failing@sha256:38183d6c842d60ed815c5fea258ffb419aa0c7980e4fcd6dcbfff1ec222bf313, test:rust/crates/urzua-core/src/rules/back_pointer.rs#a_marker_after_code_on_the_same_line_is_read_observed_failing@sha256:5c7ca68a818f72c5ffa18f0910a8e07f484b82abe46d909403ac005630e87302, code:rust/crates/urzua-core/src/rules/back_pointer.rs#embodiment_back_pointer_decision_accepted@sha256:bf81bce302370384a1d93f59073ba9a62e77edd2b6d206b81d72e5d054bceb95, test:rust/crates/urzua-core/src/rules/back_pointer.rs#the_gate_judges_every_id_of_a_noted_list_and_a_trailing_marker_observed_failing@sha256:325728feace45c5712f195e4685b645df23128fce01946d63a73a6693b65257f, test:rust/crates/urzua-core/src/rules/back_pointer.rs#a_marker_that_is_not_a_comment_marker_is_not_read@sha256:748c825d2592b38b0928632c9ca939566cb5f0eb76189b9ed9d598aa79cfdac0, test:rust/crates/urzua-core/src/rules/back_pointer.rs#reading_stops_at_a_segment_that_does_not_open_with_an_id@sha256:eebc524c75383eb124c7d743543da22a26e84e0f3cb4a36875b2acecf07bc54f, code:.urzua/config.yaml#rules::embodiment.back-pointer-decision-accepted@sha256:56679eb2875cc6314671e99bb4bbf1debd01d7842dce991c241e7fff4bfcc48c, code:rust/crates/urzua-cli/src/main.rs, code:rust/crates/urzua-cli/src/discovery.rs
---
# 147 — A back-pointer matcher that reads a full id list and trailing-line markers

## What

Make the back-pointer matcher continue past a non-id token (`Implements: ADR-1 (note), ADR-2` must yield both ids) and accept a marker that follows code on the same line, not only whole-line comments. Apply the same reading to `embodiment.back-pointer-decision-accepted`.

## Why

On a real adopter corpus the matcher stops at the first non-id token and ignores same-line markers: dozens of file-and-record pairs across dozens of files are lost, and several records lose all of their code evidence. A reader that silently drops a citation reports drift that is not there, and the same narrowness makes the decision-accepted check weaker than it looks. Evidence the author wrote must be read as written. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

- An id list with a parenthetical note between ids yields every id. — evidence: code:rust/crates/urzua-core/src/rules/back_pointer.rs#scan_marker@sha256:00c7620d0730f97c044bc524253a99a171f956587671595a2b19b7358e92abee, test:rust/crates/urzua-core/src/rules/back_pointer.rs#an_id_list_continues_past_a_parenthetical_note_observed_failing@sha256:38183d6c842d60ed815c5fea258ffb419aa0c7980e4fcd6dcbfff1ec222bf313
- A marker after code on the same line is read. — evidence: test:rust/crates/urzua-core/src/rules/back_pointer.rs#a_marker_after_code_on_the_same_line_is_read_observed_failing@sha256:5c7ca68a818f72c5ffa18f0910a8e07f484b82abe46d909403ac005630e87302
- `embodiment.back-pointer-decision-accepted` judges the same widened set. — evidence: code:rust/crates/urzua-core/src/rules/back_pointer.rs#embodiment_back_pointer_decision_accepted@sha256:bf81bce302370384a1d93f59073ba9a62e77edd2b6d206b81d72e5d054bceb95, test:rust/crates/urzua-core/src/rules/back_pointer.rs#the_gate_judges_every_id_of_a_noted_list_and_a_trailing_marker_observed_failing@sha256:325728feace45c5712f195e4685b645df23128fce01946d63a73a6693b65257f
- Tests cover each shape and a negative case that is not a marker. — evidence: test:rust/crates/urzua-core/src/rules/back_pointer.rs#a_marker_that_is_not_a_comment_marker_is_not_read@sha256:748c825d2592b38b0928632c9ca939566cb5f0eb76189b9ed9d598aa79cfdac0, test:rust/crates/urzua-core/src/rules/back_pointer.rs#reading_stops_at_a_segment_that_does_not_open_with_an_id@sha256:eebc524c75383eb124c7d743543da22a26e84e0f3cb4a36875b2acecf07bc54f
- The capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests. — evidence: code:.urzua/config.yaml#rules::embodiment.back-pointer-decision-accepted@sha256:56679eb2875cc6314671e99bb4bbf1debd01d7842dce991c241e7fff4bfcc48c, code:rust/crates/urzua-cli/src/main.rs, code:rust/crates/urzua-cli/src/discovery.rs

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Matcher widened (`SPEC-2` `0.29`): a note between ids no longer ends the list, a trailing comment after code is read, `embodiment.back-pointer-decision-accepted` judges the same set; exercised by this repository's own back-pointers. **Why:** the Done-when asks for exactly this. Status unchanged; the maintainer closes. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #193: the back-pointer matcher reads a full id list and a trailing comment after code. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
