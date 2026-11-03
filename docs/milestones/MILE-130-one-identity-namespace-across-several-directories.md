---
Stable-Id: 01M6DCRZ14207ZVE6ZN5YH6J33
Status: Planned
Phase: '0'
Track: schema-governance
---
# 130 — One identity namespace across several directories

## What

A record type may declare more than one directory under one prefix, with numbering (`urzua new`),
resolution and `identity.collision` all spanning every one of those directories as a single
namespace.

## Why

Config load now refuses two types that share a prefix (BUG-173), because a shared prefix guarantees
duplicate ids. But monorepos with per-package decision directories, archived-record folders, and
corpora mid-migration legitimately want one numbering namespace across several directories, and
today a type has exactly one `dir`, so those layouts have no correct configuration.

Related: MILE-98 (declared identity, where the namespace is defined) and BUG-168 (two types sharing
one `dir`, the inverse layout problem).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** BUG-173 makes config load refuse a shared prefix, which leaves multi-directory layouts with no correct configuration. | **substantive** |
