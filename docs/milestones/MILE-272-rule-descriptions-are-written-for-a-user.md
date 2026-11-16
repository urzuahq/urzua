---
Stable-Id: 01M7FW79MKR6FBXK0MTJDG9479
Status: Planned
Phase: '1'
Track: user-docs
Blocked-on: null
---
# 272 — Rule descriptions are written for a user

## What

Rewrite the rule descriptions shown by `urzua rules` and `explain --rule` as one or two sentences on what the rule checks and how to fix a finding, with the history moved to the records.

## Why

A walkthrough of 2026-11-15 followed only the user documentation with the released binary and found the rule descriptions to be long paragraphs full of this repository's own record ids, with the option a team needs buried inside.

## Done means

- Each rule description is under a stated length and cites no record id of this repository, enforced by a test on the rule table.
- Each rule lists its options with one line each.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
