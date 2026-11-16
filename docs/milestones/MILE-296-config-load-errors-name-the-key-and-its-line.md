---
Stable-Id: 01M7HW2FJT8W3HGJ2XAS2FQYQF
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 296 — Config load errors name the key and its line

## What

Make every configuration load error name the offending key and the line it is on.

## Why

Found on 2026-11-16 while urzua's own builders and reviewers used the tool to land a week of pull requests. A value of the wrong type under a rule is reported with the line of the first entry under `rules:` and no key name, which in a configuration of several hundred lines does not say what to fix.

## Done means

- A wrong-typed value, an unknown key and a value out of range each produce an error naming the key path and the line of that key, shown by a test per kind.
- The error's `next_action` or message says what a valid value looks like.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from friction met while using urzua on its own repository on 2026-11-16; the maintainer said that day that issues met in using and adopting the tool are improvement points. | **structural** |
