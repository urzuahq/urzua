---
Stable-Id: 01M33XB9G6FBCM194E0PKP97YD
Status: Done
Phase: '1'
Track: schema-governance
Implements: RFC-41, ADR-100
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

- The rule `code.dangling-bug-reference` reads tracked source files whose extensions config declares and reports a comment citing a record id (`BUG-\d+`, `ADR-\d+`, `RFC-\d+` per the type prefixes) whose record does not exist under that type's declared `dir`. — evidence: code:rust/crates/urzua-core/src/rules/code_citation.rs#code_dangling_bug_reference@sha256:34f54e7747d7d3aaa4497d2590e59ab52bd1a648967aeef095a26e0b0f7a288f, test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#a_dangling_citation_is_reported_with_its_line_and_a_live_one_is_not@sha256:833f814edaf076b9f391a6f37f1afbc5e14f5b8752e53b5de4aba5ba5cda4d86, test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#only_files_with_a_declared_extension_are_read@sha256:7a4f91c894c6cc3c74b3318e350564dd46f878c28eb53648202039fa4bf72f53
- The extensions and prefixes come from config, and so do the comment leaders, with the back-pointer rules' set as the default. — evidence: test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#declared_comment_prefixes_replace_the_built_in_leaders_for_their_extension@sha256:f24bb80c517830c25accd2f77e2daf163946c9610193ccfa876ee993347f6cef, test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#an_undeclared_option_keeps_the_built_in_leaders@sha256:abfb7e98ba9f48253d682050a1136448ab25b03de04530fd80614ec55eab0b58, test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#a_malformed_code_comment_prefixes_declaration_is_refused_at_load@sha256:8152eda83858d59cf79514377dea0f12acd196d2442c39ac4f3ae7be4d7f6e83, test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#the_prefixes_come_from_the_declared_record_types@sha256:06dbe7ef50d9e448a3d5d1bb9b9f2b39bfb919b6fad8d5c8baaccaff8ef38673, test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#the_rule_needs_extensions_and_refuses_one_written_with_a_dot@sha256:2711e12791cad18481b0f205002e0452478f57a39ce8b05132f881569aad4aac
- A fixture covers a dangling and a live citation. — evidence: test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#a_dangling_citation_is_reported_with_its_line_and_a_live_one_is_not@sha256:833f814edaf076b9f391a6f37f1afbc5e14f5b8752e53b5de4aba5ba5cda4d86
- The behaviour on a string literal after a leader, a doc-comment example and a URL is pinned by tests. — evidence: test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#a_string_literal_after_a_leader_and_a_doc_example_are_read_and_the_marker_excuses_them@sha256:35050f91a5bc9d17ddd5101b898d44ba1b01793c75acc84dd3efb61f3299ae60, test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#an_id_inside_a_url_in_a_comment_is_not_a_citation@sha256:f75ed52ed55f428d164b6771677b35ce43e537e74cb39bc3a7e36c485354347c
- The settlements of `RFC-41`'s open questions are recorded in an ADR and the maintainer has accepted it. `ADR-100` is Accepted (2026-11-14). — evidence: code:docs/adr/ADR-100-a-source-comment-citing-a-record-is-checked-against-the-corpus-declared-comment-leaders-existence-only-opt-in.md@sha256:7800c7f63d9a398cc4038a9c7d931c39cbd4f3fd3f0d9419ef311dfa192aa472
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
> | 2026-11-13 | Reopened: not delivered in full until `ADR-100` is Accepted. The maintainer decided on 2026-11-13 that comment leaders are declared (`code_comment_prefixes`, per extension, the back-pointer rules' set when undeclared), which replaces the earlier row's "no new key" and is built; an id inside a URL is skipped; string literals and doc examples are read and pinned by tests. `ADR-100` records the four settlements (declared leaders, string literals read with `ignore_marker` as the escape, prose and `.md` out of scope, existence only) as Proposed, decided by the maintainer on 2026-11-13 for the leaders and proposed for the rest, pending the maintainer's review. A Done-means item names that acceptance and no evidence, so the status computes to partly delivered. `Implements` now also names `ADR-100`. **Why:** `docs/rfc/README.md`: building part of an RFC gets its own ADR, and `RFC-41` asked for the questions to be settled in one before the rule ships. | **substantive** |
> | 2026-11-14 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (6 of 6 items resolved) computes to Implemented` -- stated `InProgress` disagreed with the computed value `Done`. | **structural** |
> | 2026-11-14 | Delivered in full: `ADR-100` was accepted by the maintainer on 2026-11-14, and the Done-means item naming that acceptance now pins the ADR file. **Why:** the maintainer accepted the four settlements as written, so nothing the item waited on remains. | **substantive** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/rules/code_citation.rs#code_dangling_bug_reference` (lines 4-96) 133644ae186f -> 34f54e7747d7. Reason: merge of origin/main into the MILE-207 branch: the pinned files differ from main's pins only by this branch's path-scoped source read (scoped sources in discovery.rs, check.rs, back_pointer.rs, runner.rs, table.rs, report.rs, fix.rs) and the 'not judged' notice wording; each record's own claim is untouched **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
