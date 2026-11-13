---
Stable-Id: 01M33XB9G6FBCM194E0PKP97YD
Status: Done
Phase: '1'
Track: schema-governance
Implements: RFC-41
Blocked-on: —
Non-reference-ids: BUG-104
---
# 108 — build code.dangling-bug-reference, a rule checking source comments cite real records

## What

`RFC-41`'s implementation: a new rule, `code.dangling-bug-reference`, that reads tracked source files
(extensions declared in config, per `ADR-53`) for comments citing a record id (`BUG-\d+`, `ADR-\d+`,
`RFC-\d+`, matching this project's own type prefixes) and reports one whose cited record does not
exist under that type's declared `dir`. The mirror of `embodiment.locator-exists`, which already checks
the opposite direction — a record's locator naming a real file.

## Why

Phase 0: this is a gap in the engine's own governance, found on this project's own corpus, the same
day it was found. Round 15's `BUG-104` → `BUG-108` renumbering left two source comments in `record.rs`
and `init.rs` citing the deleted `BUG-104`; nothing caught it until a human ran `grep -rn "BUG-104"` by
hand after a `/code-review` pass separately flagged one of the two instances for an unrelated reason
(temporal language) and the fix happened to notice the other while correcting it.

`RFC-41` names the open questions (comment-syntax scope, string-literal vs. doc-comment citations,
distinguishing "dangling" from "superseded," whether non-record `.md` prose is in scope) that this
milestone's implementation has to resolve, in an ADR, before shipping the rule.

## Done means

- The rule `code.dangling-bug-reference` reads tracked source files whose extensions config declares and reports a comment citing a record id (`BUG-\d+`, `ADR-\d+`, `RFC-\d+` per the type prefixes) whose record does not exist under that type's declared `dir`. — evidence: code:rust/crates/urzua-core/src/rules/code_citation.rs#code_dangling_bug_reference@sha256:423607ecb11ebc8ccf21b7dbaf41ead242f0e0df632e3c68ca92f694d90c519c, test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#a_dangling_citation_is_reported_with_its_line_and_a_live_one_is_not@sha256:833f814edaf076b9f391a6f37f1afbc5e14f5b8752e53b5de4aba5ba5cda4d86, test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#only_files_with_a_declared_extension_are_read@sha256:7a4f91c894c6cc3c74b3318e350564dd46f878c28eb53648202039fa4bf72f53
- The extensions and prefixes come from config. — evidence: test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#the_prefixes_come_from_the_declared_record_types@sha256:06dbe7ef50d9e448a3d5d1bb9b9f2b39bfb919b6fad8d5c8baaccaff8ef38673, test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#the_rule_needs_extensions_and_refuses_one_written_with_a_dot@sha256:2711e12791cad18481b0f205002e0452478f57a39ce8b05132f881569aad4aac
- A fixture covers a dangling and a live citation. — evidence: test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#a_dangling_citation_is_reported_with_its_line_and_a_live_one_is_not@sha256:833f814edaf076b9f391a6f37f1afbc5e14f5b8752e53b5de4aba5ba5cda4d86
- The rule is declared in urzua's own `.urzua/config.yaml`, and it reports nothing unwaived on this repository's `rust/` and `scripts/`. — evidence: code:.urzua/config.yaml#rules::code.dangling-bug-reference@sha256:ad0071938cc232f47ea2aa92d2d23d634641ea03ece3793d871add295a35f5a8, test:rust/crates/urzua-cli/tests/own_config_integration.rs#the_own_config_reports_a_dangling_citation_in_rust_and_scripts_and_nothing_else@sha256:bcefec2aca3309ac2af9578bf6470251039a5917b5a707cb85fa7a378f3d86b9

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Filed at Phase 0. **Why:** the incident that motivated `RFC-41` happened during this same review round, and this project's own precedent (`MILE-107`'s filing after `RFC-40`/`RFC-39`) is to schedule the work while the evidence is fresh rather than leave the RFC as an unscheduled proposal. | **substantive** |
> | 2026-11-04 | Re-phased from Phase 0 to Phase 1; `Status` unchanged. Not needed to replace the adopter's linter. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the exit is the adopter's linter deleted and urzua's own repo clean in CI, and this work is on neither path, so it moves out of Phase 0. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | Added the `Non-reference-ids` header field declaring `BUG-104`. **Why:** the text names this id as history or illustration, not as citations of a record; the declaration states that on the record itself, so a new unresolved citation in this file is reported, where a file-wide `citation.resolution` waiver had silenced every one (`BUG-190`, `ADR-75`). The field carries no decision; the milestone is unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-13 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (4 of 4 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
> | 2026-11-13 | Delivered: the rule `code.dangling-bug-reference` and its `extensions` option (`SPEC-2`, `SPEC-3`), declared in this repository's own configuration; `Done means` items now carry their pinned evidence. The four open questions of `RFC-41` were settled as follows, each additive and opt-in, so reversible without breaking a declared configuration: comment syntax is the leader set the back-pointer rules already read, with no new key (a `comment_leaders` option can be added later); a citation in a string literal after a leader is read and the declared `ignore_marker` is its escape, since the engine knows no language's string syntax; only existence is judged, so a superseded record is a live citation; prose outside the declared source extensions, including non-record `.md` files, is out of scope. **Why:** `MILE-108`; a back-pointer `embodiment.back-pointer-resolution` already judges is left to that rule so no line is reported twice, and the read is limited to the declared paths and extensions so the rule adds no scan by default (`MILE-207` scopes the existing back-pointer reads and can take this one in). | **substantive** |
