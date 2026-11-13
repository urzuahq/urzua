---
default: patch
closes: [MILE-170]
---

Finishes the corpus acceptance harness (`MILE-170`, `SPEC-4` 0.6): cases can make further commits, pass
`--base` and run in a shallow clone, and every shipped rule now has an acceptance case that fails when the
rule is switched off. New cases cover git-history drift (a shallow clone is reported, never read as no
drift), the diff-gate rules, the published-path boundary, `citation.issue-exists` against a snapshot,
`status.delivery-agreement` and the permanent ceiling of structural checks. No change to `urzua` itself.
