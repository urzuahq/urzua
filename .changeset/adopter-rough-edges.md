---
default: patch
closes: []
---

Fixes found while switching an existing repository to urzua. A source comment marker whose id is followed by a colon (`Verifies: ADR-12: why`) is read as a back-pointer, and a marker that names an id in a form urzua cannot read (`Implements: (ADR-12)`) is reported as a finding (`back-pointer-unread`) instead of being skipped. `urzua waive --from-report` quotes the findings it drafts in a fenced block, so a drafted waiver no longer reports the dangling ids it quotes. `urzua init` names `effective_requires: [Reason, Expires]` (or `permanent`) as part of the waiver declaration it withholds. The harness hooks and the record commands find a binary kept at `.urzua/bin/urzua`, and `urzua refs refresh` requests pages four at a time when GitHub names the last page.
