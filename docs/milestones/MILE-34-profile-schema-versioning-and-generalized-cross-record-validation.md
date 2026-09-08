---
Status: Planned
Stable-Id: 01M1Y5JYB0J2X7P0AVRJKWJAJV
Phase: '1'
Track: schema-governance
Implements: RFC-6
Blocked-on: —
---
# 34 — Profile schema versioning and generalized cross-record validation

## What

Version the profile schema itself (not just individual records), and generalize the ad hoc validation check/fix already does (reciprocity, dangling references) into a stated, pluggable model.

## Why

RFC-6 versions records; nothing versions the schema those records are validated against -- an explicitly open question in research with no answer yet.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
