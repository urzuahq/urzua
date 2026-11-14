---
default: patch
closes: []
---

Fixes found while switching an existing repository to urzua. A source comment marker whose id is followed by a colon (`Verifies: ADR-12: why`) is read as a back-pointer, and a marker that names an id in a form urzua cannot read (`Implements: (ADR-12)`) is counted and listed in an info notice instead of being skipped; the new rule option `unread_markers: report` makes each one a finding. `urzua waive --from-report` quotes the findings it drafts in a fenced block, so a drafted waiver no longer reports the dangling ids it quotes. `urzua init` names `effective_requires: [Reason, Expires]` (or `permanent`) as part of the waiver declaration it withholds. The harness hooks, the record commands and the session-start check find a binary kept at `.urzua/bin/urzua`, and the edit tools refuse an agent's write to `.urzua/bin`, and `urzua refs refresh` requests pages four at a time when GitHub names the last page.
