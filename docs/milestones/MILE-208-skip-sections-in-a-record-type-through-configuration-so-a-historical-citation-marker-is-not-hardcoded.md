---
Stable-Id: 01M6SSWFYC9647TWXR9KSTXE2C
Status: Planned
Phase: '1'
Track: section-checks
Blocked-on: null
---
# 208 — Skip sections in a record type through configuration, so a historical citation marker is not hardcoded

## What

Add a configuration option that lets a record type skip named sections, so a record can mark a section as historical and `citation.path-exists` does not fail on a path that no longer exists. Today only `skip_revision_log` exists, and it covers one section.

## Why

An adopter with several hundred records cites files that were later deleted or renamed, in sections that record history. The only escape is a waiver per record, because the one skip option covers the revision log alone (adopter end-to-end second pass, M7). The option changes the configuration surface, so the design is an ADR question; this record only names the need.

## Done means

- A record type can name sections that rules skip, in `.urzua/config.yaml`, as structured config and not a string mini-language (`ADR-64`).
- An unknown section name, or one that matches no heading in the type, is reported and never a silent no-op.
- `citation.path-exists` does not report a citation inside a skipped section, and reports the same citation outside one. A test fails before the change.
- `skip_revision_log` keeps its result byte for byte.
- This repository exercises the option in `.urzua/config.yaml` on a record that cites a removed path.
- `SPEC-3` documents the option with a revision row.
- A minor changeset names the option.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Initial record. **Why:** deferred during Phase 0 hardening and tracked by no record. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
