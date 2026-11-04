---
Stable-Id: 01M6JKNQ689T5T5AZDFZ3C71JQ
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
---
# 150 — A declaration excluding non-record files from a record directory

## What

Add a configuration declaration that excludes named non-record files (for example an index `README.md`) from a record type's directory.

## Why

A `README.md` in a record directory is read as a record, so `header.required-fields`, `filename.identity-pattern` and `section.required` all fire on it: about a dozen findings on a real adopter corpus, none fixable by editing the file without making it a fake record. A type must be able to say which files in its directory are not its records. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A declared exclusion removes the file from every rule's population and from the type's file census; an undeclared stray file is still reported; the exclusion is documented in the spec that owns the type declaration; and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
