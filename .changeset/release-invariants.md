---
default: patch
---

Fixes BUG-93: `make ci` checks two facts about the release that nothing read before — that the
manifest version matches the newest tag while changesets are pending, and that a changeset describing
a breaking change declares `major`. Pre-1.0 a `minor` fragment ships as a patch, so the second one
would otherwise release a config-breaking change with no Breaking Changes section.
