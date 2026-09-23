---
default: minor
---

New opt-in rule `field.leading-reserved-indicator`: flags any declared field whose value starts with a
reserved YAML indicator character (`@`, `*`, `&`, `!`, `%`, `|`, `>`), which needs quoting to parse at
all. `BUG-18` found this happening to every hand-typed `Author`/`Deciders` value in this corpus (a
purely decorative `@`, forcing avoidable quoting) via a one-off backfill with no guard against it
recurring -- which it did, twice, the same day as the backfill itself. This rule generalizes the check
past those two fields to any declared field, so the next instance is caught by `check` rather than found
by hand again.
