---
Status: Done
Stable-Id: 01M1Y5JVK0Z71FE3H8BG58VM39
Phase: '2'
Track: escalation
Implements: ADR-99
Depends-on: MILE-47
Blocked-on: —
---
# 29 — A read-only MCP server

## What

Expose rejected/superseded decisions to agents over MCP.

## Why

No RFC or ADR mentions MCP at all yet; so whether and how to expose records over it is a decision to make first.

## Done means

- An MCP server exposes rejected and superseded decisions read-only to agents, serving a record's status, reasons and supersession chain from the corpus. The generic status filter of `urzua_list_records` meets the rejected-and-superseded wording: those are the query `status: ["Rejected", "Superseded"]`, so the server knows no status name (`ADR-99`). — evidence: test:rust/crates/urzua-cli/tests/mcp_integration.rs#list_records_serves_a_rejected_decision_and_a_superseded_one_with_its_chain@sha256:380746df5768f0eddb6804517b77521c70fb1054999dc2eb01a0a648f9f0c3c9, test:rust/crates/urzua-cli/tests/mcp_integration.rs#show_record_returns_the_record_with_its_sections_text_and_supersession_edges@sha256:9c77ff02c7797264f360f0fc0f7a15ddcb64a7eb04d0beda9d33774c9307369b
- The server cannot write. — evidence: test:rust/crates/urzua-cli/tests/mcp_integration.rs#the_server_sources_and_the_code_its_tools_call_contain_no_write_api@sha256:0707b59c9e25dac1fa8e323e8ee2e07ef4551b431dff477eec162c779c5cf54e, test:rust/crates/urzua-cli/tests/mcp_integration.rs#the_working_tree_and_git_hash_the_same_after_every_tool@sha256:c19e2f03e52e2808e6d4ac3c13c9e1da08efe9d49e929d8038c36c7df0769329, test:rust/crates/urzua-cli/tests/harness_hardening_integration.rs#a_planted_write_named_urzua_mcp_tool_is_judged_as_a_write@sha256:73f39ff5cf18b8c8b57133074005ebb5df32f21a1d2c9d134c320bc5b6ca3983
- A test lists a rejected decision over the server, and a write attempt is refused. — evidence: test:rust/crates/urzua-cli/tests/mcp_integration.rs#list_records_serves_a_rejected_decision_and_a_superseded_one_with_its_chain@sha256:380746df5768f0eddb6804517b77521c70fb1054999dc2eb01a0a648f9f0c3c9, test:rust/crates/urzua-cli/tests/mcp_integration.rs#a_write_attempt_is_refused_and_the_tree_is_unchanged@sha256:cd8e9589e653f07e0a42f0cfabca7b957f1cda2b627478e28dfd690015e873f2
- An ADR decides the server's shape first, since no record mentions MCP yet. — evidence: code:docs/adr/ADR-99-urzua-ships-a-read-only-mcp-server-as-a-stdio-transport-over-its-existing-reports.md@sha256:cebd6a67fb88f6139999702c92ab04a657a0823d7c0bf2ddbaa931adadaa5c92

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | `Depends-on: MILE-47` added. **Why:** `MILE-47` states the MCP position must be decided before the read-only server is built, and `MILE-29`'s Done means requires an ADR to decide the server's shape first. Proposed for maintainer review; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-09 | Removed an appeal to research a reader cannot open; the argument now rests on the absence of any RFC or ADR on MCP. `Status` unchanged. | **structural** |
> | 2026-11-13 | Blank line removed after the revision-log header row, so the tool can append rows. **Why:** `urzua reverify` and `fix` refused to append to a log whose table was split by a blank line. | **structural** |
> | 2026-11-13 | `Implements: ADR-99` added and the Done means items given their evidence; the first item now says the generic status query of `urzua_list_records` meets its "rejected and superseded" wording. **Why:** the read-only MCP server is built (`#492`, `#500`) and `ADR-99` decides its shape; the generic filter keeps status names out of the engine (`ADR-61`). | **structural** |
> | 2026-11-13 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (4 of 4 items resolved) computes to Implemented` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
