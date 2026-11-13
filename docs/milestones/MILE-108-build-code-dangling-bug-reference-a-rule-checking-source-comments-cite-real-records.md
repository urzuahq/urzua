---
Stable-Id: 01M33XB9G6FBCM194E0PKP97YD
Status: Planned
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

- The rule `code.dangling-bug-reference` reads tracked source files whose extensions config declares and reports a comment citing a record id (`BUG-\d+`, `ADR-\d+`, `RFC-\d+` per the type prefixes) whose record does not exist under that type's declared `dir`. — evidence: code:rust/crates/urzua-core/src/rules/code_citation.rs#code_dangling_bug_reference, test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#a_dangling_citation_is_reported_with_its_line_and_a_live_one_is_not, test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#only_files_with_a_declared_extension_are_read
- The extensions and prefixes come from config. — evidence: test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#the_prefixes_come_from_the_declared_record_types, test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#the_rule_needs_extensions_and_refuses_one_written_with_a_dot
- A fixture covers a dangling and a live citation. — evidence: test:rust/crates/urzua-cli/tests/code_dangling_reference_integration.rs#a_dangling_citation_is_reported_with_its_line_and_a_live_one_is_not
- The rule is declared in urzua's own `.urzua/config.yaml`, and it reports nothing unwaived on this repository's `rust/` and `scripts/`. — evidence: code:.urzua/config.yaml#rules::code.dangling-bug-reference, test:rust/crates/urzua-cli/tests/own_config_integration.rs#the_own_config_reports_a_dangling_citation_in_rust_and_scripts_and_nothing_else

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Filed at Phase 0. **Why:** the incident that motivated `RFC-41` happened during this same review round, and this project's own precedent (`MILE-107`'s filing after `RFC-40`/`RFC-39`) is to schedule the work while the evidence is fresh rather than leave the RFC as an unscheduled proposal. | **substantive** |
> | 2026-11-04 | Re-phased from Phase 0 to Phase 1; `Status` unchanged. Not needed to replace the adopter's linter. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the exit is the adopter's linter deleted and urzua's own repo clean in CI, and this work is on neither path, so it moves out of Phase 0. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | Added the `Non-reference-ids` header field declaring `BUG-104`. **Why:** the text names this id as history or illustration, not as citations of a record; the declaration states that on the record itself, so a new unresolved citation in this file is reported, where a file-wide `citation.resolution` waiver had silenced every one (`BUG-190`, `ADR-75`). The field carries no decision; the milestone is unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
