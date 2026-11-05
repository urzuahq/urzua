---
default: minor
closes: [MILE-188]
---

The editor hook records a named override in a committed log. A top-level `override_log` declares the file, a Markdown table of date, name, record, field, rule, reason and a blank `Reviewed-by` that a person fills in. The new opt-in rule `override.unreviewed` warns on every entry whose `Reviewed-by` does not name a person (a blank cell, a placeholder such as `TBD` or `-`, and a declared pending value are not a review), so an override surfaces in the pull request that carries it, and reports a row that is not an entry and a declared log that does not exist. With a log declared the hook needs a reason (`--reason`, `URZUA_HOOK_REASON`) and refuses an override it cannot record.
