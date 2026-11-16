---
Stable-Id: 01M7J21M7BY5HS7KB5JKAF33MP
Status: Planned
Phase: '1'
Track: schema-governance
Blocked-on: null
---
# 308 — Engine names carry no workflow's vocabulary

## What

Rename the commands, rule ids, config keys, report keys and harness names that carry one workflow's words to neutral terms, with no alias for the old names: the change is breaking and ships in one release, and `urzua migrate config` converts a configuration to the new names.

## Why

An audit on 2026-11-16 found no record type or status word driving engine behaviour, but found generic mechanisms named for one workflow: waivers, deciders, decisions, delivery, a roadmap, supersession, a Y-statement, a spec per type. The maintainer's position is that the tooling has no opinion about which record types a repository defines. The maintainer chose each new name that day.

## Done means

- The command that drafts suppression records is `urzua suppress`; the rule is `suppression.matches-nothing` and the flag `--assist-suppressions`; the maintainer chose "urzua suppress".
- The rule `code.dangling-bug-reference` is `code.dangling-reference`.
- The authority model says approver: config `authority.approvers`, rule `diff.approver-required`, findings `gated-...-without-approver`.
- `embodiment.requires-accepted-status` is `embodiment.requires-success-status`, `embodiment.back-pointer-decision-accepted` is `embodiment.back-pointer-target-implementable`, and the option `decision_types` is `gated_types`.
- The checklist mechanism says checklist: config `statuses.checklist` and `checklist.items`, rules `status.checklist-agreement`, `status.checklist-shape` and `status.checklist-item-stale`.
- The planning command is `urzua plan` with config block `plan:`; the report keeps `ready` and `critical_path` and names its ordered groups `stages`.
- Supersession is an ordinary declared relation pair: the built-in role and `relation.supersession-reciprocity` are gone, and `urzua migrate config` converts a configuration that used them.
- The section shape `y-statement` is `required-phrases`.
- The per-type key `spec:` is `described_by:` and the rule `type.no-declared-spec` is `type.no-description-record`; it stays opt-in.
- The harness guardrail `acceptance` is `gated-status`; the commands `/amend` and `/done` keep their names.
- No old name is kept as an alias: an old command, rule id or config key is refused with a message naming the new one, shown by a test per name, and `urzua migrate config` rewrites a configuration to the new names.
- A decision record states the renames and that the release is breaking, the changelog lists every old and new name, and the reference pages, specs and this repository's own configuration use the new names.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** The maintainer said on 2026-11-16 "rename them to neutral terms" and chose each name in answer to ten questions asked one at a time that day. | **structural** |
> | 2026-11-16 | No aliases: the renames ship as one breaking release. **Why:** The maintainer decided the same day: "you don't have to keep around things for a release, like old commands and aliases. We're happy with the breaking release"; the one other repository that uses urzua is updated as soon as that release is published. | **substantive** |
